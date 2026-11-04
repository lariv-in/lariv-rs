//! Uniform capability shape: hooks + items, infallible [`Capability::mount`] → [`Tagged`].
//!
//! Every Lariv capability follows the same builder pattern: a [`CapStore`] holds deferred
//! plugin hooks and initial items; at mount time hooks fold over items and the result is
//! wrapped in a [`Tagged`] value on the app's capability HList.
//!
//! # Core types
//!
//! - [`HasCapTag`] — tag identity for builder-phase stores
//! - [`Capability`] — mount contract (`hooks` + `items` → [`Tagged`])
//! - [`CapStore`] — shared hooks/items container keyed by phantom `Tag`
//! - [`ApplyHooks`] — fold hook HList over items (tail first = install order)
//! - [`CapHookExt`] — prepend plugin hooks on custom builder capabilities (`cap_hook` install step)
//! - [`FoldMount`] — fold a builder HList of capabilities into mounted outputs
//! - [`UnitHookProof`] — proof tuple for hook lists that use the default `()` proof, so
//!   [`Capability::mount`] does not take a caller-supplied proof parameter
//!
//! # Registrar capabilities
//!
//! Item-list capabilities (templates, slots, migrations, commands) use [`RegistrarItems`],
//! [`FoldRegistrarHooks`], and `apply_registrar_hook!` (defined in `lariv-rs-macros`).
//! Opaque capabilities
//! ([`crate::apps::AppsCapability`], the LLM tools registry, …) implement
//! dedicated [`ApplyHooks`] impls instead. Deployment-local capabilities use a local struct
//! with [`Capability`] + [`CapHookExt`] and register via `cap_attach` / `cap_hook` in
//! [`define_plugin_install!`](crate::plugin_install::define_plugin_install).
//!
//! # Plugin macros
//!
//! Defined in `lariv-rs-macros` and re-exported here:
//!
//! - [`define_register_items!`] — declare item tags + `Register*` hook impl
//! - [`define_replace_templates!`] — replace tagged templates at compile-time indices
//! - [`define_passthrough_cap!`] — mount items as plain [`Tagged::new`](Tagged::new)(items)
//!
//! # Examples
//!
//! ```rust ignore
//! // Passthrough state capability (no hooks):
//! define_passthrough_cap!(UsersStateCap, UsersTag, UsersState);
//!
//! // Template items + register hook:
//! define_register_items! {
//!     plugin: UsersTag;
//!     capability: TemplateCapability;
//!     trait: RegisterTemplates;
//!     method: register_templates;
//!     wrapper: TemplateOf;
//!     bounds: [Clone, ProvideRequestCaps, Send, Sync];
//!     hook: RegisterTemplatesHook;
//!     items: [LoginIdx: UsersLoginPageTag => LoginPage]
//! }
//! ```

use std::marker::PhantomData;

use frunk::{HCons, HNil};

use crate::tag::Tagged;

/// Tag identity for builder-phase capability stores (independent of [`Capability`]).
///
/// Lookup/`add_hook` work on stores that do not yet implement [`Capability`] (e.g. Http
/// with pending mount-route hooks).
pub trait HasCapTag {
    type Tag;
}

/// Prepend a plugin hook onto a builder-phase capability (for
/// [`define_plugin_install!`](crate::plugin_install::define_plugin_install) `cap_hook`).
pub trait CapHookExt<Plugin, Hook>: HasCapTag + Sized {
    type Hooked: HasCapTag<Tag = Self::Tag>;
    fn prepend_cap_hook(self, hook: Hook) -> Self::Hooked;
}

impl<Tag, Hooks, Items, Plugin, Hook> CapHookExt<Plugin, Hook> for CapStore<Tag, Hooks, Items> {
    type Hooked = CapStore<Tag, HCons<Tagged<Plugin, Hook>, Hooks>, Items>;

    fn prepend_cap_hook(self, hook: Hook) -> Self::Hooked {
        self.add_hook::<Plugin, Hook>(hook)
    }
}

impl<Tag, Hooks, Items> HasCapTag for CapStore<Tag, Hooks, Items> {
    type Tag = Tag;
}

/// Builder-phase capability: deferred hooks applied to items, then folded to [`Tagged`].
pub trait Capability: HasCapTag + Sized {
    /// Always `Tagged<Self::Tag, Self::Value>`.
    type Output;
    type Value;
    type Hooks;
    type Items;

    fn mount(self) -> Self::Output;
}

/// Shared hooks + items store keyed by `Tag`.
#[derive(Clone, Debug)]
pub struct CapStore<Tag, Hooks, Items> {
    pub hooks: Hooks,
    pub items: Items,
    _tag: PhantomData<fn() -> Tag>,
}

impl<Tag, Hooks, Items> CapStore<Tag, Hooks, Items> {
    /// Construct a store with explicit hooks and items.
    ///
    /// Use when a capability starts with a non-empty hook HList (e.g. after
    /// [`add_hook`](Self::add_hook) in a plugin install chain).
    ///
    /// # Examples
    ///
    /// ```rust ignore
    /// let cap = CapStore::<MyTag, _, _>::new(my_hooks, my_items);
    /// ```
    pub fn new(hooks: Hooks, items: Items) -> Self {
        Self {
            hooks,
            items,
            _tag: PhantomData,
        }
    }

    /// Construct a store with default (empty) hooks and the given items.
    ///
    /// Typical for capabilities whose hooks start as [`HNil`] and are prepended during
    /// plugin install via [`add_hook`](Self::add_hook).
    pub fn empty_hooks(items: Items) -> Self
    where
        Hooks: Default,
    {
        Self::new(Hooks::default(), items)
    }
}

impl<Tag, Items> CapStore<Tag, HNil, Items> {
    /// Construct a hook-free store (common entry point for [`with_*`](crate::db::with_db) helpers).
    ///
    /// Equivalent to `CapStore::new(HNil, items)`.
    ///
    /// # Examples
    ///
    /// ```rust ignore
    /// app.add_capability(CapStore::with_items(DbState { conn }))
    /// ```
    pub fn with_items(items: Items) -> Self {
        Self::new(HNil, items)
    }
}

/// Marker for plugin tags whose [`CapStore`] with [`HNil`] hooks mounts as plain [`Tagged::new`](Tagged::new)(items).
pub trait PassthroughCapTag {}

impl<Tag, Items> Capability for CapStore<Tag, HNil, Items>
where
    Tag: PassthroughCapTag,
{
    type Value = Items;
    type Output = Tagged<Tag, Items>;
    type Hooks = HNil;
    type Items = Items;

    fn mount(self) -> Self::Output {
        Tagged::new(self.items)
    }
}

impl<Tag, Hooks, Items> CapStore<Tag, Hooks, Items> {
    /// Prepend a tagged hook.
    pub fn add_hook<HTag, H>(self, hook: H) -> CapStore<Tag, HCons<Tagged<HTag, H>, Hooks>, Items> {
        CapStore {
            hooks: HCons {
                head: Tagged::new(hook),
                tail: self.hooks,
            },
            items: self.items,
            _tag: PhantomData,
        }
    }

    /// Prepend a tagged item.
    pub fn add_item<ITag, I>(self, item: I) -> CapStore<Tag, Hooks, HCons<Tagged<ITag, I>, Items>> {
        CapStore {
            hooks: self.hooks,
            items: HCons {
                head: Tagged::new(item),
                tail: self.items,
            },
            _tag: PhantomData,
        }
    }

    /// Transform items in place, preserving hooks (e.g. prepend default commands before mount).
    ///
    /// # Examples
    ///
    /// ```rust ignore
    /// cap.map_items(|items| items.prepend::<MyCmdTag, _>(MyCommand))
    /// ```
    pub fn map_items<F, NewItems>(self, f: F) -> CapStore<Tag, Hooks, NewItems>
    where
        F: FnOnce(Items) -> NewItems,
    {
        CapStore {
            hooks: self.hooks,
            items: f(self.items),
            _tag: PhantomData,
        }
    }

    /// Transform hooks in place, preserving items.
    pub fn map_hooks<F, NewHooks>(self, f: F) -> CapStore<Tag, NewHooks, Items>
    where
        F: FnOnce(Hooks) -> NewHooks,
    {
        CapStore {
            hooks: f(self.hooks),
            items: self.items,
            _tag: PhantomData,
        }
    }
}

/// Fold hooks over items (tail first = registration / install order).
///
/// `Proof` carries frunk indices (and nested proofs from earlier hooks), matching
/// [`crate::http::FoldMountRoutes`]. Default `()` is for [`HNil`] / no-index hooks.
pub trait ApplyHooks<Items, Proof = ()>: Sized {
    type Output;
    fn apply_hooks(self, items: Items) -> Self::Output;
}

impl<Items> ApplyHooks<Items, ()> for HNil {
    type Output = Items;

    fn apply_hooks(self, items: Items) -> Self::Output {
        items
    }
}

/// Marks item types folded by [`apply_registrar_hook!`] (frunk HLists). Opaque
/// capabilities ([`crate::apps::AppsCapability`], the LLM tools registry, …)
/// use dedicated [`ApplyHooks`] impls instead.
mod registrar_items {
    pub trait Sealed {}
    impl Sealed for ::frunk::HNil {}
    impl<Head, Tail> Sealed for ::frunk::HCons<Head, Tail> where Tail: Sealed {}
}

pub trait RegistrarItems: registrar_items::Sealed {}

impl<T> RegistrarItems for T where T: registrar_items::Sealed {}

/// Proof token for [`crate::template::TemplateRegistrar`] hook folding.
pub struct TemplateHookProof;

/// Proof token for [`crate::components::SlotRegistrar`] hook folding.
pub struct SlotHookProof;

/// Proof token for [`crate::migration::MigrationRegistrar`] hook folding.
pub struct MigrationHookProof;

/// Proof token for [`crate::command::CommandRegistrar`] hook folding.
pub struct CommandHookProof;

/// Fold registrar hooks without a proof tuple (used by [`CapStore::resolve_hooks`]).
pub trait FoldRegistrarHooks<Tag, Items> {
    type Output;
    fn fold_registrar_hooks(self, items: Items) -> Self::Output;
}

impl<Tag, Items> FoldRegistrarHooks<Tag, Items> for HNil {
    type Output = Items;

    fn fold_registrar_hooks(self, items: Items) -> Self::Output {
        items
    }
}

pub(crate) use lariv_rs_macros::apply_registrar_hook;
pub use lariv_rs_macros::{
    define_passthrough_cap, define_register_items, define_replace_templates,
};

/// Proof tuple for hook lists whose each step uses [`ApplyHooks`]'s default `()` proof.
///
/// `HNil` is `()`. Each cons cell is `(Tail::Proof, ())`, matching registrar impls such as
/// [`crate::apps::AppsRegistrar`] and route hooks generated with the default
/// [`crate::http::RouteRegistrar`] proof. Naming the tuple on [`Capability`] would leave
/// the parameter unconstrained, so mount reads it from the hook list instead.
pub trait UnitHookProof {
    type Proof;
}

impl UnitHookProof for HNil {
    type Proof = ();
}

impl<Head, Tail> UnitHookProof for HCons<Head, Tail>
where
    Tail: UnitHookProof,
{
    type Proof = (Tail::Proof, ());
}

/// Apply [`FoldRegistrarHooks`] and wrap the items as the mounted [`Tagged`] value.
pub fn mount_registrar<Tag, Hooks, Items, Value, F>(
    store: CapStore<Tag, Hooks, Items>,
    wrap: F,
) -> Tagged<Tag, Value>
where
    Hooks: FoldRegistrarHooks<Tag, Items>,
    F: FnOnce(<Hooks as FoldRegistrarHooks<Tag, Items>>::Output) -> Value,
{
    let items = store.hooks.fold_registrar_hooks(store.items);
    Tagged::new(wrap(items))
}

/// Apply [`ApplyHooks`] using [`UnitHookProof`] and wrap the items as the mounted [`Tagged`] value.
pub fn mount_unit_hooks<Tag, Hooks, Items, Value, F>(
    store: CapStore<Tag, Hooks, Items>,
    wrap: F,
) -> Tagged<Tag, Value>
where
    Hooks: UnitHookProof + ApplyHooks<Items, <Hooks as UnitHookProof>::Proof>,
    F: FnOnce(<Hooks as ApplyHooks<Items, <Hooks as UnitHookProof>::Proof>>::Output) -> Value,
{
    let items = <Hooks as ApplyHooks<Items, <Hooks as UnitHookProof>::Proof>>::apply_hooks(
        store.hooks,
        store.items,
    );
    Tagged::new(wrap(items))
}

/// Fold a builder HList of [`Capability`] values into mounted [`Tagged`] outputs.
pub trait FoldMount: Sized {
    type Output;
    fn fold_mount(self) -> Self::Output;
}

impl FoldMount for HNil {
    type Output = HNil;

    fn fold_mount(self) -> Self::Output {
        HNil
    }
}

impl<Head, Tail> FoldMount for HCons<Head, Tail>
where
    Head: Capability,
    Tail: FoldMount,
{
    type Output = HCons<Head::Output, Tail::Output>;

    fn fold_mount(self) -> Self::Output {
        HCons {
            head: self.head.mount(),
            tail: self.tail.fold_mount(),
        }
    }
}

/// Helper: apply hooks then wrap items as the mounted [`Tagged`] value.
pub fn mount_with_hooks<Tag, Hooks, Items, Value, F>(
    store: CapStore<Tag, Hooks, Items>,
    wrap: F,
) -> Tagged<Tag, Value>
where
    Hooks: ApplyHooks<Items>,
    F: FnOnce(Hooks::Output) -> Value,
{
    let items = store.hooks.apply_hooks(store.items);
    Tagged::new(wrap(items))
}
