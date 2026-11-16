//! Extra entries on the admin user-detail sidebar, contributed by other plugins.
//!
//! The users plugin attaches this capability and a base hook that adds no links.
//! Addon plugins `cap_hook` it and push a [`UserDetailLink`]. The folded list is
//! stored once at app mount and read by [`user_detail_links`].

use std::marker::PhantomData;
use std::sync::OnceLock;

use frunk::{HCons, HNil};
use lariv_core::app::App;
use lariv_core::capability::{CapHookExt, Capability, HasCapTag};
use lariv_core::tag::Tagged;
use lariv_core::traits::add::{AddCapability, CapTagAbsent};

static LINKS: OnceLock<Vec<UserDetailLink>> = OnceLock::new();

/// One extra entry in the admin user detail sidebar, after the built-in items.
#[derive(Clone, Debug)]
pub struct UserDetailLink {
    /// Active-state key compared against the `active` argument of `user_detail_menu`.
    pub key: &'static str,
    pub title: &'static str,
    /// Builds the link target for one user id.
    pub url_fn: fn(user_id: i64) -> String,
    pub order: u16,
}

/// User detail link registry folded from base and addon hooks.
#[derive(Clone, Debug, Default)]
pub struct UserDetailMenuRegistry {
    links: Vec<UserDetailLink>,
}

impl UserDetailMenuRegistry {
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    /// Add a link. A duplicate `key` is ignored.
    pub fn push(mut self, link: UserDetailLink) -> Self {
        if !self.links.iter().any(|existing| existing.key == link.key) {
            self.links.push(link);
        }
        self
    }

    pub fn links(&self) -> &[UserDetailLink] {
        &self.links
    }

    pub(crate) fn sorted_links(self) -> Vec<UserDetailLink> {
        let mut links = self.links;
        links.sort_by_key(|link| (link.order, link.title));
        links
    }
}

/// Capability tag for the user detail sidebar registry.
pub struct UserDetailMenuTag;

/// Builder-phase registry. Addon plugins prepend hooks with `cap_hook`.
#[derive(Clone, Default)]
pub struct UserDetailMenuCap<Hooks> {
    pub hooks: Hooks,
    pub items: UserDetailMenuRegistry,
    _tag: PhantomData<fn() -> UserDetailMenuTag>,
}

impl<Hooks> UserDetailMenuCap<Hooks> {
    pub fn new() -> Self
    where
        Hooks: Default,
    {
        Self {
            hooks: Hooks::default(),
            items: UserDetailMenuRegistry::new(),
            _tag: PhantomData,
        }
    }

    pub fn add_hook<HTag, H>(self, hook: H) -> UserDetailMenuCap<HCons<Tagged<HTag, H>, Hooks>> {
        UserDetailMenuCap {
            hooks: HCons {
                head: Tagged::new(hook),
                tail: self.hooks,
            },
            items: self.items,
            _tag: PhantomData,
        }
    }
}

impl<Hooks> HasCapTag for UserDetailMenuCap<Hooks> {
    type Tag = UserDetailMenuTag;
}

impl<Hooks, Plugin, Hook> CapHookExt<Plugin, Hook> for UserDetailMenuCap<Hooks> {
    type Hooked = UserDetailMenuCap<HCons<Tagged<Plugin, Hook>, Hooks>>;

    fn prepend_cap_hook(self, hook: Hook) -> Self::Hooked {
        self.add_hook::<Plugin, Hook>(hook)
    }
}

/// Plugin hook that contributes user-detail sidebar links.
pub trait UserDetailMenuRegistrar: Sized {
    fn register_user_detail_links(self, cap: UserDetailMenuRegistry) -> UserDetailMenuRegistry;
}

/// Fold registrar hooks over the registry. Tail first, so install order wins.
pub trait FoldUserDetailMenuHooks {
    fn fold(self, registry: UserDetailMenuRegistry) -> UserDetailMenuRegistry;
}

impl FoldUserDetailMenuHooks for HNil {
    fn fold(self, registry: UserDetailMenuRegistry) -> UserDetailMenuRegistry {
        registry
    }
}

impl<Plugin, H, Tail> FoldUserDetailMenuHooks for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: FoldUserDetailMenuHooks,
    H: UserDetailMenuRegistrar + Copy,
{
    fn fold(self, registry: UserDetailMenuRegistry) -> UserDetailMenuRegistry {
        let registry = self.tail.fold(registry);
        self.head.value.register_user_detail_links(registry)
    }
}

impl<Hooks> Capability for UserDetailMenuCap<Hooks>
where
    Hooks: FoldUserDetailMenuHooks,
{
    type Value = UserDetailMenuRegistry;
    type Output = Tagged<UserDetailMenuTag, UserDetailMenuRegistry>;
    type Hooks = Hooks;
    type Items = UserDetailMenuRegistry;

    fn mount(self) -> Self::Output {
        let registry = self.hooks.fold(self.items);
        store_user_detail_links(&registry);
        Tagged::new(registry)
    }
}

/// Attach an empty user-detail menu capability.
///
/// Prefer `cap_attach` in the users plugin install steps.
pub fn with_user_detail_menu<L, Proof>(app: App<L>) -> App<HCons<UserDetailMenuCap<HNil>, L>>
where
    L: frunk::hlist::HList + CapTagAbsent<UserDetailMenuTag, Proof>,
{
    app.add_capability(UserDetailMenuCap::new())
}

fn store_user_detail_links(registry: &UserDetailMenuRegistry) {
    if LINKS.set(registry.clone().sorted_links()).is_err() {
        tracing::error!("user detail menu LINKS already initialized");
    }
}

/// Registered user-detail links, sorted by order.
///
/// Before app mount this falls back to the base hook, which registers nothing.
pub fn user_detail_links() -> Vec<UserDetailLink> {
    match LINKS.get() {
        Some(links) => links.clone(),
        None => BaseHook
            .register_user_detail_links(UserDetailMenuRegistry::new())
            .sorted_links(),
    }
}

/// Base hook installed by the users plugin. Built-in items stay in the template.
#[derive(Clone, Copy, Default)]
pub struct BaseHook;

impl UserDetailMenuRegistrar for BaseHook {
    fn register_user_detail_links(self, cap: UserDetailMenuRegistry) -> UserDetailMenuRegistry {
        cap
    }
}
