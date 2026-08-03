//! Stub Rune environment registry when `cap-llm` is disabled (empty capability only).

use std::sync::Arc;

use frunk::{HCons, HNil, hlist::HList};
use sea_orm::DatabaseConnection;
use serde_json::Value as JsonValue;

use crate::{
    app::App,
    capability::{ApplyHooks, CapStore, Capability, mount_with_hooks},
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};

/// Placeholder when the filesystem plugin is not enabled.
pub struct StubFilestore;

/// Capability tag for the Rune script environment registry.
pub struct RuneEnvTag;

/// Request-time context when resolving contextual bindings.
pub struct RuneEnvCtx<'a> {
    pub db: &'a DatabaseConnection,
    pub store: Arc<StubFilestore>,
}

pub type NativeFn = Arc<
    dyn for<'a> Fn(&RuneEnvCtx<'a>, &[JsonValue]) -> Result<JsonValue, String> + Send + Sync,
>;

/// Resolved native binding (static JSON value or callable).
pub enum NativeBinding {
    Value(JsonValue),
    Function(NativeFn),
}

/// Runtime registry of Rune environment entries.
#[derive(Clone, Default)]
pub struct RuneEnvCapability {
    bindings: Vec<(String, JsonValue)>,
}

impl RuneEnvCapability {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_static(&mut self, name: impl Into<String>, value: JsonValue) -> &mut Self {
        let name = name.into();
        if let Some(existing) = self.bindings.iter_mut().find(|(n, _)| *n == name) {
            existing.1 = value;
        } else {
            self.bindings.push((name, value));
        }
        self
    }

    pub fn register_contextual<F>(&mut self, _name: impl Into<String>, _factory: F) -> &mut Self
    where
        F: for<'a> Fn(&RuneEnvCtx<'a>) -> NativeBinding + Send + Sync + 'static,
    {
        self
    }

    pub fn all_names(&self) -> Vec<String> {
        self.bindings.iter().map(|(n, _)| n.clone()).collect()
    }

    pub fn resolve(&self, _ctx: &RuneEnvCtx<'_>) -> ResolvedRuneEnv {
        ResolvedRuneEnv {
            statics: self.bindings.clone(),
            functions: Vec::new(),
        }
    }
}

/// Bindings materialized for a single script run.
pub struct ResolvedRuneEnv {
    pub statics: Vec<(String, JsonValue)>,
    pub functions: Vec<(String, NativeFn)>,
}

/// Plugin hook for appending bindings onto a [`RuneEnvCapability`].
pub trait RuneEnvRegistrar {
    fn register_rune_env(self, rune_env: &mut RuneEnvCapability);
}

pub type RuneEnvCap<Hooks> = CapStore<RuneEnvTag, Hooks, RuneEnvCapability>;

impl<Hooks> RuneEnvCap<Hooks> {
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
    &[]
}
