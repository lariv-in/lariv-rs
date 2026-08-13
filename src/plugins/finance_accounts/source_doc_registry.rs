//! Source document type capability — install-time registration via `cap_hook`.
//!
//! Hub attaches [`SourceDocCap`]; invoices/creditnotes register loaders with
//! `cap_hook(SourceDocTag, SourceDocCap, Hook)`. Handlers extract
//! [`Cap`](crate::http::Cap)`<`[`SourceDocRegistry`]`>`.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::{
    app::App,
    capability::{CapHookExt, Capability, HasCapTag},
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use frunk::{HCons, HNil, hlist::HList};
use sea_orm::DatabaseConnection;

/// Capability tag for the source document type registry.
pub struct SourceDocTag;

/// Loaded backing document for a resolved type/id pair.
pub trait SourceDocInstance: Send + Sync {
    fn source_doc_type(&self) -> &str;
    fn source_doc_id(&self) -> i64;
    fn display_name(&self) -> String;
    fn detail_url(&self) -> String;
}

/// Describes how one document kind participates in linking and URLs.
#[async_trait]
pub trait SourceDocType: Send + Sync {
    fn source_doc_type(&self) -> &str;
    fn display_name(&self) -> &str;
    fn detail_url(&self, id: i64) -> String;
    async fn load_from_id(
        &self,
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<Arc<dyn SourceDocInstance>>;
}

/// Folded map of registered source document type loaders (mounted capability value).
#[derive(Clone, Default)]
pub struct SourceDocRegistry {
    types: HashMap<String, Arc<dyn SourceDocType>>,
}

impl SourceDocRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a loader; duplicate keys are ignored (install-order first wins).
    pub fn register(mut self, loader: Arc<dyn SourceDocType>) -> Self {
        let key = loader.source_doc_type().to_string();
        self.types.entry(key).or_insert(loader);
        self
    }

    pub fn get(&self, typ: &str) -> Option<Arc<dyn SourceDocType>> {
        self.types.get(typ).cloned()
    }

    /// Type display label from a registered loader, or a humanized fallback.
    pub fn type_display_name(&self, typ: &str) -> String {
        self.get(typ)
            .map(|loader| loader.display_name().to_string())
            .unwrap_or_else(|| humanize_type_name(typ))
    }

    /// Detail route for a type/id pair when the type is registered.
    pub fn type_detail_url(&self, typ: &str, id: i64) -> Option<String> {
        self.get(typ).map(|loader| loader.detail_url(id))
    }

    pub async fn resolve_instance(
        &self,
        db: &DatabaseConnection,
        typ: &str,
        id: i64,
    ) -> Result<Arc<dyn SourceDocInstance>> {
        if typ.is_empty() {
            return Err(anyhow!(
                "p_finance_accounts: ResolveSourceDocInstance: empty type"
            ));
        }
        let loader = self.get(typ).ok_or_else(|| {
            anyhow!("p_finance_accounts: ResolveSourceDocInstance: unknown type {typ:?}")
        })?;
        let inst = loader.load_from_id(db, id).await?;
        if inst.source_doc_type() != typ {
            return Err(anyhow!(
                "p_finance_accounts: ResolveSourceDocInstance: type mismatch: registry key {typ:?}, instance {:?}",
                inst.source_doc_type()
            ));
        }
        Ok(inst)
    }
}

/// Plugin hook for registering source document types at install time.
pub trait SourceDocRegistrar: Sized {
    fn register_source_docs(self, registry: SourceDocRegistry) -> SourceDocRegistry;
}

/// Builder-phase source document capability.
#[derive(Clone, Default)]
pub struct SourceDocCap<Hooks> {
    pub hooks: Hooks,
    pub items: SourceDocRegistry,
    _tag: PhantomData<fn() -> SourceDocTag>,
}

impl<Hooks> SourceDocCap<Hooks> {
    pub fn new() -> Self
    where
        Hooks: Default,
    {
        Self {
            hooks: Hooks::default(),
            items: SourceDocRegistry::new(),
            _tag: PhantomData,
        }
    }

    pub fn add_hook<HTag, H>(self, hook: H) -> SourceDocCap<HCons<Tagged<HTag, H>, Hooks>> {
        SourceDocCap {
            hooks: HCons {
                head: Tagged::new(hook),
                tail: self.hooks,
            },
            items: self.items,
            _tag: PhantomData,
        }
    }
}

impl<Hooks> HasCapTag for SourceDocCap<Hooks> {
    type Tag = SourceDocTag;
}

impl<Hooks, Plugin, Hook> CapHookExt<Plugin, Hook> for SourceDocCap<Hooks> {
    type Hooked = SourceDocCap<HCons<Tagged<Plugin, Hook>, Hooks>>;

    fn prepend_cap_hook(self, hook: Hook) -> Self::Hooked {
        self.add_hook::<Plugin, Hook>(hook)
    }
}

/// Fold registrar hooks over the registry (tail first = install order).
pub trait FoldSourceDocRegistrarHooks {
    fn fold(self, registry: SourceDocRegistry) -> SourceDocRegistry;
}

impl FoldSourceDocRegistrarHooks for HNil {
    fn fold(self, registry: SourceDocRegistry) -> SourceDocRegistry {
        registry
    }
}

impl<Plugin, H, Tail> FoldSourceDocRegistrarHooks for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: FoldSourceDocRegistrarHooks,
    H: SourceDocRegistrar + Copy,
{
    fn fold(self, registry: SourceDocRegistry) -> SourceDocRegistry {
        let registry = self.tail.fold(registry);
        self.head.value.register_source_docs(registry)
    }
}

impl<Hooks> Capability for SourceDocCap<Hooks>
where
    Hooks: FoldSourceDocRegistrarHooks,
{
    type Value = SourceDocRegistry;
    type Output = Tagged<SourceDocTag, SourceDocRegistry>;
    type Hooks = Hooks;
    type Items = SourceDocRegistry;

    fn mount(self) -> Self::Output {
        let registry = self.hooks.fold(self.items);
        Tagged::new(registry)
    }
}

/// No-op base hook from the accounts hub plugin.
#[derive(Clone, Copy, Default)]
pub struct BaseHook;

impl SourceDocRegistrar for BaseHook {
    fn register_source_docs(self, registry: SourceDocRegistry) -> SourceDocRegistry {
        registry
    }
}

/// Attach an empty source-doc capability (prefer `cap_attach` in install steps).
pub fn with_source_docs<L, Proof>(app: App<L>) -> App<HCons<SourceDocCap<HNil>, L>>
where
    L: HList + CapTagAbsent<SourceDocTag, Proof>,
{
    app.add_capability(SourceDocCap::<HNil>::new())
}

/// Humanize an unregistered type key's last path segment (e.g. `SomeDocument` → `Some document`).
pub fn humanize_type_name(typ: &str) -> String {
    let name = typ.rsplit('.').next().unwrap_or(typ);
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push(' ');
        }
        out.extend(ch.to_lowercase());
    }
    if out.is_empty() {
        typ.to_string()
    } else {
        let mut chars = out.chars();
        match chars.next() {
            None => typ.to_string(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_type_humanizes() {
        assert_eq!(
            humanize_type_name("p_example.SomeDocument"),
            "Some document"
        );
    }

    #[test]
    fn type_display_name_falls_back() {
        let reg = SourceDocRegistry::new();
        assert_eq!(
            reg.type_display_name("p_example.SomeDocument"),
            "Some document"
        );
    }
}
