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

/// Registers assistant HITL filesystem helpers onto the HITL capability.
#[derive(Clone, Copy, Default)]
pub struct Hook;

impl HitlRegistrar for Hook {
    fn register_hitl(self, hitl: &mut HitlCapability) {
        register(hitl);
    }
}

fn register(hitl: &mut HitlCapability) {
    hitl.register(
        "delete_vnode",
        "delete_vnode(#{ path: string } | #{ id: int }) -> ()  // requires human approval; deletes a file or directory and all descendants",
        |_ctx| NativeBinding::Function(Arc::new(delete_vnode)),
    );
}

fn delete_vnode(ctx: &RuneEnvCtx<'_>, args: &[rune::Value]) -> Result<rune::Value, String> {
    use super::rune_env::{parse_vnode_ref, resolve_any_vnode};
    use crate::plugins::filesystem::node;
    use crate::rune_env::{block_on_async, rune_to_json};

    let value = args
        .first()
        .ok_or_else(|| "delete_vnode requires an object argument".to_string())?;
    let parsed = parse_vnode_ref(&rune_to_json(value)?, "delete_vnode")?;
    let db = ctx.db.clone();
    let store = Arc::clone(&ctx.store);
    block_on_async(async move {
        let (vnode, _) = resolve_any_vnode(&db, parsed).await?;
        node::delete_tree(&db, store.as_ref(), &vnode)
            .await
            .map_err(|e| e.to_string())
    })?;
    Ok(rune::Value::from(()))
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

    fn test_env_ctx<'a>(
        db: &'a sea_orm::DatabaseConnection,
        store: &'a std::sync::Arc<crate::plugins::filesystem::storage::DynFilestore>,
    ) -> RuneEnvCtx<'a> {
        RuneEnvCtx {
            db,
            store: std::sync::Arc::clone(store),
            session_id: None,
        }
    }

    fn registered_hitl() -> HitlCapability {
        let mut cap = HitlCapability::new();
        Hook.register_hitl(&mut cap);
        cap
    }

    #[test]
    fn registers_delete_vnode() {
        let names = registered_hitl().all_names();
        assert!(
            names.iter().any(|name| name == "delete_vnode"),
            "expected delete_vnode in {names:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn delete_vnode_rejects_missing_path_or_id() {
        use crate::plugins::filesystem::storage::{DynFilestore, UnimplementedFilestore};
        use crate::plugins::llm_assistant::rune_engine::{self, CompileOpts};
        use crate::rune_env::RuneEnvCapability;

        let hitl = registered_hitl();
        let rune = RuneEnvCapability::new();
        let gate = approve_all_gate();
        let db = sea_orm::DatabaseConnection::default();
        let store: std::sync::Arc<DynFilestore> = std::sync::Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run_with(
            &rune,
            &env_ctx,
            "delete_vnode(#{})",
            &[],
            CompileOpts {
                hitl: Some(&hitl),
                hitl_gate: Some(&gate),
            },
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("path or id is required"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn delete_vnode_rejects_path_and_id() {
        use crate::plugins::filesystem::storage::{DynFilestore, UnimplementedFilestore};
        use crate::plugins::llm_assistant::rune_engine::{self, CompileOpts};
        use crate::rune_env::RuneEnvCapability;

        let hitl = registered_hitl();
        let rune = RuneEnvCapability::new();
        let gate = approve_all_gate();
        let db = sea_orm::DatabaseConnection::default();
        let store: std::sync::Arc<DynFilestore> = std::sync::Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run_with(
            &rune,
            &env_ctx,
            r#"delete_vnode(#{ path: "/docs/a.txt", id: 1 })"#,
            &[],
            CompileOpts {
                hitl: Some(&hitl),
                hitl_gate: Some(&gate),
            },
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("either path or id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn delete_vnode_rejects_missing_argument() {
        use crate::plugins::filesystem::storage::{DynFilestore, UnimplementedFilestore};
        use crate::plugins::llm_assistant::rune_engine::{self, CompileOpts};
        use crate::rune_env::RuneEnvCapability;

        let hitl = registered_hitl();
        let rune = RuneEnvCapability::new();
        let gate = approve_all_gate();
        let db = sea_orm::DatabaseConnection::default();
        let store: std::sync::Arc<DynFilestore> = std::sync::Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run_with(
            &rune,
            &env_ctx,
            "delete_vnode(())",
            &[],
            CompileOpts {
                hitl: Some(&hitl),
                hitl_gate: Some(&gate),
            },
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("delete_vnode requires an object argument")
                || error.contains("unsupported")
                || error.contains("path or id"),
            "unexpected error payload: {out}"
        );
    }
}
