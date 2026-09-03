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
//!         env.register_static("app_version", "app_version: string constant", json!("1.0"));
//!         env.register_contextual(
//!             "current_user",
//!             "current_user() -> #{ id: int, name: string }",
//!             |ctx| NativeBinding::Value(json!(lookup_user(ctx.db))),
//!         );
//!     }
//! }
//!
//! let app = with_rune_env(app);
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use frunk::{HCons, HNil, hlist::HList};
use rune::Value;
use sea_orm::DatabaseConnection;
use serde_json::{Value as JsonValue, json};

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
    /// Active assistant conversation, when the script runs inside a chat turn.
    pub session_id: Option<i64>,
}

/// Kind of a registered Rune environment identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuneBindingKind {
    Static,
    Function,
}

impl RuneBindingKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Function => "function",
        }
    }
}

/// Name, kind, and schema/docs for one Rune environment identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuneBindingInfo {
    pub name: String,
    pub kind: RuneBindingKind,
    pub schema: String,
}

pub type NativeFn =
    Arc<dyn for<'a> Fn(&RuneEnvCtx<'a>, &[Value]) -> Result<Value, String> + Send + Sync>;

/// Drive an async future from a sync Rune native function.
///
/// `block_in_place` only works on the multi-thread runtime. Deployments such as
/// Uniquity use a current-thread runtime (large-stack install/mount), so this
/// falls back to a dedicated thread with its own runtime.
pub fn block_on_async<T, F>(fut: F) -> T
where
    T: Send,
    F: std::future::Future<Output = T> + Send,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(|| handle.block_on(fut))
        }
        Ok(_) | Err(_) => std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("rune native runtime")
                        .block_on(fut)
                })
                .join()
                .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
        }),
    }
}

/// Resolved native binding (static JSON value or callable).
pub enum NativeBinding {
    Value(JsonValue),
    Function(NativeFn),
}

type ContextualFactory = Arc<dyn for<'a> Fn(&RuneEnvCtx<'a>) -> NativeBinding + Send + Sync>;

#[derive(Clone)]
enum StoredBindingKind {
    Static(JsonValue),
    Contextual(ContextualFactory),
}

#[derive(Clone)]
struct StoredBinding {
    kind: StoredBindingKind,
    /// Human-readable signature / docs for the skill Content field hint.
    doc: String,
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
    ///
    /// `doc` is shown on the skill Content field hint when this binding is mounted.
    pub fn register_static(
        &mut self,
        name: impl Into<String>,
        doc: impl Into<String>,
        value: JsonValue,
    ) -> &mut Self {
        self.upsert(
            name.into(),
            StoredBinding {
                kind: StoredBindingKind::Static(value),
                doc: doc.into(),
            },
        );
        self
    }

    /// Register a request-scoped binding factory (evaluated at [`Self::resolve`] time).
    ///
    /// `doc` should describe the Rune call signature and return shape (used by the skill
    /// Content field hint for the bindings present in this deployment).
    pub fn register_contextual<F>(
        &mut self,
        name: impl Into<String>,
        doc: impl Into<String>,
        factory: F,
    ) -> &mut Self
    where
        F: for<'a> Fn(&RuneEnvCtx<'a>) -> NativeBinding + Send + Sync + 'static,
    {
        self.upsert(
            name.into(),
            StoredBinding {
                kind: StoredBindingKind::Contextual(Arc::new(factory)),
                doc: doc.into(),
            },
        );
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

    /// Look up a registered identifier's kind and schema/docs.
    pub fn lookup(&self, name: &str) -> Option<RuneBindingInfo> {
        self.bindings
            .iter()
            .find(|(n, _)| n == name)
            .map(|(n, b)| RuneBindingInfo {
                name: n.clone(),
                kind: match b.kind {
                    StoredBindingKind::Static(_) => RuneBindingKind::Static,
                    StoredBindingKind::Contextual(_) => RuneBindingKind::Function,
                },
                schema: b.doc.clone(),
            })
    }

    /// Documentation strings for registered bindings (registration order).
    ///
    /// Empty docs are omitted. Used to patch the skill Content field hint from mounted plugins.
    pub fn binding_docs(&self) -> Vec<&str> {
        self.bindings
            .iter()
            .map(|(_, b)| b.doc.as_str())
            .filter(|d| !d.is_empty())
            .collect()
    }

    /// Resolve static + contextual bindings for one tool invocation.
    pub fn resolve(&self, ctx: &RuneEnvCtx<'_>) -> ResolvedRuneEnv {
        let mut statics = Vec::new();
        let mut functions = Vec::new();
        for (name, binding) in &self.bindings {
            match &binding.kind {
                StoredBindingKind::Static(v) => statics.push((name.clone(), v.clone())),
                StoredBindingKind::Contextual(factory) => match factory(ctx) {
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

/// Convert a Rune VM value into JSON (objects, arrays, and scalars).
pub fn rune_to_json(v: &Value) -> Result<JsonValue, String> {
    rune_to_json_at(v, "$")
}

fn rune_to_json_at(v: &Value, path: &str) -> Result<JsonValue, String> {
    if v.into_unit().is_ok() {
        return Ok(JsonValue::Null);
    }
    if let Ok(b) = v.as_bool() {
        return Ok(json!(b));
    }
    if let Ok(n) = v.as_integer::<i64>() {
        return Ok(json!(n));
    }
    if let Ok(n) = v.as_unsigned() {
        return Ok(json!(n));
    }
    if let Ok(n) = v.as_float() {
        return Ok(json!(n));
    }
    if let Ok(c) = v.as_char() {
        return Ok(json!(c.to_string()));
    }
    if let Ok(s) = rune::from_value::<String>(v.clone()) {
        return Ok(json!(s));
    }
    if let Ok(vec) = v.borrow_ref::<rune::runtime::Vec>() {
        let arr: Result<Vec<_>, _> = vec
            .iter()
            .enumerate()
            .map(|(i, item)| rune_to_json_at(item, &format!("{path}[{i}]")))
            .collect();
        return Ok(JsonValue::Array(arr?));
    }
    if let Ok(obj) = v.borrow_ref::<rune::runtime::Object>() {
        let mut map = serde_json::Map::new();
        for (k, val) in obj.iter() {
            map.insert(k.to_string(), rune_to_json_at(val, &format!("{path}.{k}"))?);
        }
        return Ok(JsonValue::Object(map));
    }
    if let Ok(tuple) = v.borrow_tuple_ref() {
        if tuple.is_empty() {
            return Ok(JsonValue::Null);
        }
        let items: Result<Vec<_>, _> = tuple
            .iter()
            .enumerate()
            .map(|(i, item)| rune_to_json_at(item, &format!("{path}[{i}]")))
            .collect();
        return Ok(JsonValue::Array(items?));
    }
    Err(format!(
        "unsupported Rune type `{}` at {path}",
        v.type_info()
    ))
}

/// Convert JSON into a Rune VM value. Objects become `#{}` maps, not strings.
pub fn json_to_rune(value: JsonValue) -> Result<Value, String> {
    match value {
        JsonValue::Null => Ok(Value::from(())),
        JsonValue::Bool(b) => Ok(Value::from(b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::from(i))
            } else if let Some(u) = n.as_u64() {
                if let Ok(i) = i64::try_from(u) {
                    Ok(Value::from(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(Value::from(f))
                } else {
                    Err("number out of range".into())
                }
            } else if let Some(f) = n.as_f64() {
                Ok(Value::from(f))
            } else {
                Err("unsupported number".into())
            }
        }
        JsonValue::String(s) => rune::to_value(s).map_err(|e| e.to_string()),
        JsonValue::Array(items) => {
            let vals: Result<Vec<_>, _> = items.into_iter().map(json_to_rune).collect();
            rune::to_value(vals?).map_err(|e| e.to_string())
        }
        JsonValue::Object(map) => {
            let mut out = HashMap::new();
            for (k, v) in map {
                out.insert(k, json_to_rune(v)?);
            }
            rune::to_value(out).map_err(|e| e.to_string())
        }
    }
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
        cap.register_static("pi", "pi: float", JsonValue::from(3.14));
        cap.register_static("pi", "pi: float (updated)", JsonValue::from(3.14159));
        assert_eq!(cap.all_names(), vec!["pi".to_string()]);
        assert_eq!(cap.binding_docs(), vec!["pi: float (updated)"]);
        let info = cap.lookup("pi").expect("pi");
        assert_eq!(info.kind, RuneBindingKind::Static);
        assert_eq!(info.schema, "pi: float (updated)");
        assert!(cap.lookup("missing").is_none());
    }

    #[tokio::test]
    async fn block_on_async_works_on_current_thread() {
        let n = block_on_async(async { 7i64 });
        assert_eq!(n, 7);
    }

    #[test]
    fn json_object_roundtrips_as_object_not_string() {
        let src = json!({
            "site": { "id": 1, "name": "Aayush Developer" },
            "purchase_orders": [{ "id": 8, "number": "P26RIN100294" }],
        });
        let rune = json_to_rune(src.clone()).expect("json_to_rune");
        let back = rune_to_json(&rune).expect("rune_to_json");
        assert_eq!(back, src);
        assert!(back.is_object());
    }

    #[test]
    fn unsigned_integer_converts_to_json() {
        let v = Value::from(7u64);
        assert_eq!(rune_to_json(&v).expect("unsigned"), json!(7));
    }
}
