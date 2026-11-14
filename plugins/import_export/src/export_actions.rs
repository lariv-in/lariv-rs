//! Extra download forms on the export page.
//!
//! Plugins register an [`ExportPageAction`] with `cap_hook` on
//! [`ExportPageActionsCap`]. The export page renders each contributor under
//! the workbook form.

use std::marker::PhantomData;
use std::sync::Arc;

use async_trait::async_trait;
use frunk::{HCons, HNil};
use maud::{Markup, html};

use lariv_core::capability::{CapHookExt, Capability, HasCapTag};
use lariv_core::tag::Tagged;

/// Capability tag for extra export-page actions.
pub struct ExportPageActionsTag;

/// Renders extra markup on the export page.
#[async_trait]
pub trait ExportPageAction: Send + Sync {
    async fn render(&self) -> Markup;
}

/// Contributors folded at mount and invoked on each export page.
#[derive(Clone, Default)]
pub struct ExportPageActionRegistry {
    contributors: Vec<Arc<dyn ExportPageAction>>,
}

impl ExportPageActionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(mut self, action: Arc<dyn ExportPageAction>) -> Self {
        self.contributors.push(action);
        self
    }

    pub async fn render(&self) -> Markup {
        let mut combined = Markup::default();
        for contributor in &self.contributors {
            let piece = contributor.render().await;
            combined = html! {
                (combined)
                (piece)
            };
        }
        combined
    }
}

/// Plugin hook that appends export-page contributors.
pub trait ExportPageActionRegistrar: Sized {
    fn register_export_actions(
        self,
        registry: ExportPageActionRegistry,
    ) -> ExportPageActionRegistry;
}

/// Builder-phase export-action capability.
#[derive(Clone, Default)]
pub struct ExportPageActionsCap<Hooks> {
    pub hooks: Hooks,
    pub items: ExportPageActionRegistry,
    _tag: PhantomData<fn() -> ExportPageActionsTag>,
}

impl<Hooks> ExportPageActionsCap<Hooks> {
    pub fn new() -> Self
    where
        Hooks: Default,
    {
        Self {
            hooks: Hooks::default(),
            items: ExportPageActionRegistry::new(),
            _tag: PhantomData,
        }
    }

    pub fn add_hook<HTag, H>(self, hook: H) -> ExportPageActionsCap<HCons<Tagged<HTag, H>, Hooks>> {
        ExportPageActionsCap {
            hooks: HCons {
                head: Tagged::new(hook),
                tail: self.hooks,
            },
            items: self.items,
            _tag: PhantomData,
        }
    }
}

impl<Hooks> HasCapTag for ExportPageActionsCap<Hooks> {
    type Tag = ExportPageActionsTag;
}

impl<Hooks, Plugin, Hook> CapHookExt<Plugin, Hook> for ExportPageActionsCap<Hooks> {
    type Hooked = ExportPageActionsCap<HCons<Tagged<Plugin, Hook>, Hooks>>;

    fn prepend_cap_hook(self, hook: Hook) -> Self::Hooked {
        self.add_hook::<Plugin, Hook>(hook)
    }
}

/// Fold registrar hooks over the registry (tail first = install order).
pub trait FoldExportPageActionHooks {
    fn fold(self, registry: ExportPageActionRegistry) -> ExportPageActionRegistry;
}

impl FoldExportPageActionHooks for HNil {
    fn fold(self, registry: ExportPageActionRegistry) -> ExportPageActionRegistry {
        registry
    }
}

impl<Plugin, H, Tail> FoldExportPageActionHooks for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: FoldExportPageActionHooks,
    H: ExportPageActionRegistrar + Copy,
{
    fn fold(self, registry: ExportPageActionRegistry) -> ExportPageActionRegistry {
        let registry = self.tail.fold(registry);
        self.head.value.register_export_actions(registry)
    }
}

impl<Hooks> Capability for ExportPageActionsCap<Hooks>
where
    Hooks: FoldExportPageActionHooks,
{
    type Value = ExportPageActionRegistry;
    type Output = Tagged<ExportPageActionsTag, ExportPageActionRegistry>;
    type Hooks = Hooks;
    type Items = ExportPageActionRegistry;

    fn mount(self) -> Self::Output {
        Tagged::new(self.hooks.fold(self.items))
    }
}
