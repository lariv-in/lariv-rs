//! Rune environment registry — plugins register bindings for assistant scripts.
//!
//! Mirrors [`crate::llm_tools::LlmToolsCapability`]: deferred registrar hooks at mount,
//! mounted as [`Arc<RuneEnvCapability>`] for cheap request-extension clones.
//!
//! Plugins expose static JSON values or request-scoped native functions that Rune scripts
//! can call during LLM Assistant tool runs.
//!
//! # Lifecycle
//!
//! 1. Attach via [`with_rune_env`].
//! 2. Plugins implement [`RuneEnvRegistrar`] to call [`RuneEnvCapability::register_static`]
//!    or [`RuneEnvCapability::register_contextual`].
//! 3. At mount, hooks fold → [`Arc<RuneEnvCapability>`] on the app HList.
//! 4. Tool handlers call [`RuneEnvCapability::resolve`] with [`RuneEnvCtx`] per invocation.
//!
//! # Core types
//!
//! - [`RuneEnvTag`] — capability tag
//! - [`RuneEnvCapability`] — mounted binding registry
//! - [`RuneEnvCtx`] — request-time context for contextual bindings
//! - [`NativeBinding`] / [`NativeFn`] — resolved callable or static value
//! - [`ResolvedRuneEnv`] — materialized bindings for one script run
//! - [`RuneEnvCap`] — builder-phase [`CapStore`]
//! - [`RuneEnvRegistrar`] — plugin hook trait
//!
//! # Examples
//!
//! ```rust ignore
//! impl RuneEnvRegistrar for RegisterRuneEnvHook {
//!     fn register_rune_env(self, env: &mut RuneEnvCapability) {
//!         env.register_static("app_version", json!("1.0"));
//!         env.register_contextual("current_user", |ctx| {
//!             NativeBinding::Value(json!(lookup_user(ctx.db)))
//!         });
//!     }
//! }
//!
//! let app = with_rune_env(app);
//! ```

use std::sync::Arc;

use frunk::{HCons, HNil, hlist::HList};
use rune::Value;
use sea_orm::DatabaseConnection;
use serde_json::Value as JsonValue;

use crate::{
    app::App,
    capability::{ApplyHooks, CapStore, Capability, mount_with_hooks},
    plugins::filesystem::storage::DynFilestore,
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};

/// Capability tag for the Rune script environment registry.
pub struct RuneEnvTag;

/// Request-time context when resolving contextual bindings.
pub struct RuneEnvCtx<'a> {
    pub db: &'a DatabaseConnection,
    pub store: Arc<DynFilestore>,
}

pub type NativeFn =
    Arc<dyn for<'a> Fn(&RuneEnvCtx<'a>, &[Value]) -> Result<Value, String> + Send + Sync>;

/// Resolved native binding (static JSON value or callable).
pub enum NativeBinding {
    Value(JsonValue),
    Function(NativeFn),
}

type ContextualFactory = Arc<dyn for<'a> Fn(&RuneEnvCtx<'a>) -> NativeBinding + Send + Sync>;

#[derive(Clone)]
enum StoredBinding {
    Static(JsonValue),
    Contextual(ContextualFactory),
}

/// Runtime registry of Rune environment entries.
#[derive(Clone, Default)]
pub struct RuneEnvCapability {
    bindings: Vec<(String, StoredBinding)>,
}

impl RuneEnvCapability {
    /// Empty binding registry (starting point for [`RuneEnvRegistrar`] hooks).
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a static JSON value (same for every request).
    pub fn register_static(&mut self, name: impl Into<String>, value: JsonValue) -> &mut Self {
        self.upsert(name.into(), StoredBinding::Static(value));
        self
    }

    /// Register a request-scoped binding factory (evaluated at [`Self::resolve`] time).
    pub fn register_contextual<F>(&mut self, name: impl Into<String>, factory: F) -> &mut Self
    where
        F: for<'a> Fn(&RuneEnvCtx<'a>) -> NativeBinding + Send + Sync + 'static,
    {
        self.upsert(name.into(), StoredBinding::Contextual(Arc::new(factory)));
        self
    }

    fn upsert(&mut self, name: String, binding: StoredBinding) {
        if let Some(existing) = self.bindings.iter_mut().find(|(n, _)| *n == name) {
            existing.1 = binding;
        } else {
            self.bindings.push((name, binding));
        }
    }

    /// All registered binding names (static and contextual).
    pub fn all_names(&self) -> Vec<String> {
        self.bindings.iter().map(|(n, _)| n.clone()).collect()
    }

    /// Resolve static + contextual bindings for one tool invocation.
    pub fn resolve(&self, ctx: &RuneEnvCtx<'_>) -> ResolvedRuneEnv {
        let mut statics = Vec::new();
        let mut functions = Vec::new();
        for (name, binding) in &self.bindings {
            match binding {
                StoredBinding::Static(v) => statics.push((name.clone(), v.clone())),
                StoredBinding::Contextual(factory) => match factory(ctx) {
                    NativeBinding::Value(v) => statics.push((name.clone(), v)),
                    NativeBinding::Function(f) => functions.push((name.clone(), f)),
                },
            }
        }
        ResolvedRuneEnv { statics, functions }
    }
}

/// Bindings materialized for a single script run.
pub struct ResolvedRuneEnv {
    pub statics: Vec<(String, JsonValue)>,
    pub functions: Vec<(String, NativeFn)>,
}

/// Plugin hook for appending bindings onto a [`RuneEnvCapability`].
///
/// Must mutate in place (not chain by-value returns).
pub trait RuneEnvRegistrar {
    fn register_rune_env(self, rune_env: &mut RuneEnvCapability);
}

pub type RuneEnvCap<Hooks> = CapStore<RuneEnvTag, Hooks, RuneEnvCapability>;

impl<Hooks> RuneEnvCap<Hooks> {
    /// Eagerly fold registrar hooks into items (testing / pre-mount inspection).
    pub fn resolve_hooks<Proof>(self) -> RuneEnvCap<HNil>
    where
        Hooks: ApplyHooks<RuneEnvCapability, Proof, Output = RuneEnvCapability>,
    {
        CapStore::with_items(self.hooks.apply_hooks(self.items))
    }
}

impl<Plugin, H, Tail, TailProof> ApplyHooks<RuneEnvCapability, (TailProof, ())>
    for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: ApplyHooks<RuneEnvCapability, TailProof, Output = RuneEnvCapability>,
    H: RuneEnvRegistrar,
{
    type Output = RuneEnvCapability;

    fn apply_hooks(self, items: RuneEnvCapability) -> Self::Output {
        let mut items = self.tail.apply_hooks(items);
        self.head.value.register_rune_env(&mut items);
        items
    }
}

impl<Hooks> Capability for RuneEnvCap<Hooks>
where
    Hooks: ApplyHooks<RuneEnvCapability, (), Output = RuneEnvCapability>,
{
    type Value = Arc<RuneEnvCapability>;
    type Output = Tagged<RuneEnvTag, Arc<RuneEnvCapability>>;
    type Hooks = Hooks;
    type Items = RuneEnvCapability;

    fn mount(self) -> Self::Output {
        mount_with_hooks(self, Arc::new)
    }
}

/// Attach an empty Rune environment capability to the app builder.
pub fn with_rune_env<L, Proof>(app: App<L>) -> App<HCons<RuneEnvCap<HNil>, L>>
where
    L: HList + CapTagAbsent<RuneEnvTag, Proof>,
{
    app.add_capability(CapStore::with_items(RuneEnvCapability::new()))
}

/// Curated Rune standard-library module names (for `list_rune_env` tool output).
pub fn standard_library_names() -> &'static [&'static str] {
    &[
        "std::array",
        "std::bytes",
        "std::char",
        "std::clone",
        "std::cmp",
        "std::collections",
        "std::convert",
        "std::core",
        "std::env",
        "std::fmt",
        "std::fs",
        "std::future",
        "std::hash",
        "std::io",
        "std::iter",
        "std::math",
        "std::object",
        "std::ops",
        "std::option",
        "std::panic",
        "std::pin",
        "std::prelude",
        "std::rand",
        "std::result",
        "std::slice",
        "std::string",
        "std::tuple",
        "std::type",
        "std::vec",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_static_and_names() {
        let mut cap = RuneEnvCapability::new();
        cap.register_static("pi", JsonValue::from(3.14));
        cap.register_static("pi", JsonValue::from(3.14159));
        assert_eq!(cap.all_names(), vec!["pi".to_string()]);
    }
}
