//! Extra buttons on a vnode detail header.
//!
//! Plugins register a [`VNodeDetailAction`] with `cap_hook` on
//! [`VNodeDetailActionsCap`]. The detail handler asks each contributor for
//! markup after it knows whether the viewer may write into the node.

use std::marker::PhantomData;
use std::sync::Arc;

use async_trait::async_trait;
use frunk::{HCons, HNil};
use maud::{Markup, html};
use sea_orm::DatabaseConnection;

use lariv_core::capability::{CapHookExt, Capability, HasCapTag};
use lariv_core::tag::Tagged;

use crate::entities::VNode;

/// Capability tag for vnode detail header actions.
pub struct VNodeDetailActionsTag;

/// Renders extra header buttons for one vnode.
#[async_trait]
pub trait VNodeDetailAction: Send + Sync {
    async fn header_actions(
        &self,
        db: &DatabaseConnection,
        node: &VNode,
        can_write: bool,
    ) -> Markup;
}

/// Contributors folded at mount and invoked on each detail request.
#[derive(Clone, Default)]
pub struct VNodeDetailActionRegistry {
    contributors: Vec<Arc<dyn VNodeDetailAction>>,
}

impl VNodeDetailActionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(mut self, action: Arc<dyn VNodeDetailAction>) -> Self {
        self.contributors.push(action);
        self
    }

    pub async fn render(&self, db: &DatabaseConnection, node: &VNode, can_write: bool) -> Markup {
        let mut combined = Markup::default();
        for contributor in &self.contributors {
            let piece = contributor.header_actions(db, node, can_write).await;
            combined = html! {
                (combined)
                (piece)
            };
        }
        combined
    }
}

/// Plugin hook that appends detail-header contributors.
pub trait VNodeDetailActionRegistrar: Sized {
    fn register_detail_actions(
        self,
        registry: VNodeDetailActionRegistry,
    ) -> VNodeDetailActionRegistry;
}

/// Builder-phase detail-action capability.
#[derive(Clone, Default)]
pub struct VNodeDetailActionsCap<Hooks> {
    pub hooks: Hooks,
    pub items: VNodeDetailActionRegistry,
    _tag: PhantomData<fn() -> VNodeDetailActionsTag>,
}

impl<Hooks> VNodeDetailActionsCap<Hooks> {
    pub fn new() -> Self
    where
        Hooks: Default,
    {
        Self {
            hooks: Hooks::default(),
            items: VNodeDetailActionRegistry::new(),
            _tag: PhantomData,
        }
    }

    pub fn add_hook<HTag, H>(
        self,
        hook: H,
    ) -> VNodeDetailActionsCap<HCons<Tagged<HTag, H>, Hooks>> {
        VNodeDetailActionsCap {
            hooks: HCons {
                head: Tagged::new(hook),
                tail: self.hooks,
            },
            items: self.items,
            _tag: PhantomData,
        }
    }
}

impl<Hooks> HasCapTag for VNodeDetailActionsCap<Hooks> {
    type Tag = VNodeDetailActionsTag;
}

impl<Hooks, Plugin, Hook> CapHookExt<Plugin, Hook> for VNodeDetailActionsCap<Hooks> {
    type Hooked = VNodeDetailActionsCap<HCons<Tagged<Plugin, Hook>, Hooks>>;

    fn prepend_cap_hook(self, hook: Hook) -> Self::Hooked {
        self.add_hook::<Plugin, Hook>(hook)
    }
}

/// Fold registrar hooks over the registry (tail first = install order).
pub trait FoldVNodeDetailActionHooks {
    fn fold(self, registry: VNodeDetailActionRegistry) -> VNodeDetailActionRegistry;
}

impl FoldVNodeDetailActionHooks for HNil {
    fn fold(self, registry: VNodeDetailActionRegistry) -> VNodeDetailActionRegistry {
        registry
    }
}

impl<Plugin, H, Tail> FoldVNodeDetailActionHooks for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: FoldVNodeDetailActionHooks,
    H: VNodeDetailActionRegistrar + Copy,
{
    fn fold(self, registry: VNodeDetailActionRegistry) -> VNodeDetailActionRegistry {
        let registry = self.tail.fold(registry);
        self.head.value.register_detail_actions(registry)
    }
}

impl<Hooks> Capability for VNodeDetailActionsCap<Hooks>
where
    Hooks: FoldVNodeDetailActionHooks,
{
    type Value = VNodeDetailActionRegistry;
    type Output = Tagged<VNodeDetailActionsTag, VNodeDetailActionRegistry>;
    type Hooks = Hooks;
    type Items = VNodeDetailActionRegistry;

    fn mount(self) -> Self::Output {
        Tagged::new(self.hooks.fold(self.items))
    }
}
