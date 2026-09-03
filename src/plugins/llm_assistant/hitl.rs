//! Human-in-the-loop Rune bindings — plugins register functions that need approval.
//!
//! Mirrors [`crate::rune_env::RuneEnvCapability`]: deferred [`HitlRegistrar`] hooks at mount,
//! mounted as [`Arc<HitlCapability>`]. Scripts call these names like other Rune env functions;
//! [`crate::llm_tools::HitlGate`] blocks until a human approves (or the call fails closed).

use std::sync::Arc;

use frunk::{HCons, HNil};
use serde_json::Value as JsonValue;

use crate::{
    capability::{CapStore, Capability},
    llm_tools::{HitlGate, HitlSource},
    rune_env::{NativeBinding, NativeFn, RuneEnvCtx},
    tag::Tagged,
};

/// Capability tag for HITL-gated Rune functions.
pub struct HitlTag;

type ContextualFactory = Arc<dyn for<'a> Fn(&RuneEnvCtx<'a>) -> NativeBinding + Send + Sync>;

#[derive(Clone)]
struct StoredHitl {
    factory: ContextualFactory,
    /// Human-readable signature / docs (skill hint + `list_rune_env`).
    doc: String,
}

/// Runtime registry of HITL-gated Rune functions.
#[derive(Clone, Default)]
pub struct HitlCapability {
    bindings: Vec<(String, StoredHitl)>,
}

impl HitlCapability {
    /// Empty HITL registry (starting point for [`HitlRegistrar`] hooks).
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a request-scoped HITL function (evaluated at [`Self::resolve`] time).
    ///
    /// `doc` should describe the Rune call signature and that the call waits for approval.
    pub fn register<F>(
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
            StoredHitl {
                factory: Arc::new(factory),
                doc: doc.into(),
            },
        );
        self
    }

    fn upsert(&mut self, name: String, binding: StoredHitl) {
        if let Some(existing) = self.bindings.iter_mut().find(|(n, _)| *n == name) {
            existing.1 = binding;
        } else {
            self.bindings.push((name, binding));
        }
    }

    /// All registered HITL binding names.
    pub fn all_names(&self) -> Vec<String> {
        self.bindings.iter().map(|(n, _)| n.clone()).collect()
    }

    /// Schema/docs for a registered HITL identifier.
    pub fn lookup(&self, name: &str) -> Option<String> {
        self.bindings
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| b.doc.clone())
    }

    /// Documentation strings for registered HITL bindings (registration order).
    pub fn binding_docs(&self) -> Vec<&str> {
        self.bindings
            .iter()
            .map(|(_, b)| b.doc.as_str())
            .filter(|d| !d.is_empty())
            .collect()
    }

    /// Resolve HITL functions for one script run.
    pub fn resolve(&self, ctx: &RuneEnvCtx<'_>) -> Vec<(String, NativeFn)> {
        let mut functions = Vec::new();
        for (name, binding) in &self.bindings {
            match (binding.factory)(ctx) {
                NativeBinding::Function(f) => functions.push((name.clone(), f)),
                NativeBinding::Value(_) => {}
            }
        }
        functions
    }
}

impl HitlSource for HitlCapability {
    fn all_names(&self) -> Vec<String> {
        Self::all_names(self)
    }

    fn binding_docs(&self) -> Vec<String> {
        self.binding_docs()
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    fn lookup(&self, name: &str) -> Option<String> {
        Self::lookup(self, name)
    }

    fn resolve(&self, ctx: &RuneEnvCtx<'_>) -> Vec<(String, NativeFn)> {
        Self::resolve(self, ctx)
    }
}

/// Plugin hook for appending HITL functions onto a [`HitlCapability`].
pub trait HitlRegistrar {
    fn register_hitl(self, hitl: &mut HitlCapability);
}

/// Builder-phase HITL capability.
pub type HitlCap<Hooks> = CapStore<HitlTag, Hooks, HitlCapability>;

impl<Hooks> HitlCap<Hooks> {
    /// Eagerly fold registrar hooks into items (testing / pre-mount inspection).
    pub fn resolve_hooks(self) -> HitlCap<HNil>
    where
        Hooks: FoldHitlRegistrarHooks,
    {
        CapStore::with_items(self.hooks.fold(self.items))
    }
}

/// Fold [`HitlRegistrar`] hooks (tail first = install order).
pub trait FoldHitlRegistrarHooks {
    fn fold(self, items: HitlCapability) -> HitlCapability;
}

impl FoldHitlRegistrarHooks for HNil {
    fn fold(self, items: HitlCapability) -> HitlCapability {
        items
    }
}

impl<Plugin, H, Tail> FoldHitlRegistrarHooks for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: FoldHitlRegistrarHooks,
    H: HitlRegistrar,
{
    fn fold(self, items: HitlCapability) -> HitlCapability {
        let mut items = self.tail.fold(items);
        self.head.value.register_hitl(&mut items);
        items
    }
}

impl<Hooks> Capability for HitlCap<Hooks>
where
    Hooks: FoldHitlRegistrarHooks,
{
    type Value = Arc<HitlCapability>;
    type Output = Tagged<HitlTag, Arc<HitlCapability>>;
    type Hooks = Hooks;
    type Items = HitlCapability;

    fn mount(self) -> Self::Output {
        let items = self.hooks.fold(self.items);
        Tagged::new(Arc::new(items))
    }
}

/// Always-approve gate for tests.
pub fn approve_all_gate() -> HitlGate {
    Arc::new(|_name, _args| Ok(()))
}

/// Always-deny gate for tests.
pub fn deny_all_gate() -> HitlGate {
    Arc::new(|_name, _args| Err("denied".into()))
}

/// Convert Rune invoke arguments into JSON for the HITL UI / gate.
pub fn args_to_json(args: &[rune::Value]) -> Result<JsonValue, String> {
    use crate::rune_env::rune_to_json;
    if args.is_empty() {
        return Ok(JsonValue::Null);
    }
    if args.len() == 1 {
        return rune_to_json(&args[0]);
    }
    let items: Result<Vec<_>, _> = args.iter().map(rune_to_json).collect();
    Ok(JsonValue::Array(items?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rune_env::NativeBinding;

    #[test]
    fn register_upserts_by_name() {
        let mut cap = HitlCapability::new();
        cap.register("wipe", "wipe() // first", |_ctx| {
            NativeBinding::Function(Arc::new(|_ctx, _args| Ok(rune::Value::from(()))))
        });
        cap.register("wipe", "wipe() // updated", |_ctx| {
            NativeBinding::Function(Arc::new(|_ctx, _args| Ok(rune::Value::from(()))))
        });
        assert_eq!(cap.all_names(), vec!["wipe".to_string()]);
        assert_eq!(cap.binding_docs(), vec!["wipe() // updated"]);
        assert_eq!(cap.lookup("wipe").as_deref(), Some("wipe() // updated"));
        assert!(cap.lookup("missing").is_none());
    }

    #[test]
    fn empty_docs_omitted() {
        let mut cap = HitlCapability::new();
        cap.register("x", "", |_ctx| NativeBinding::Value(JsonValue::Null));
        assert!(cap.binding_docs().is_empty());
        assert_eq!(cap.all_names(), vec!["x".to_string()]);
    }
}
