//! Uniform capability shape: hooks + items, infallible [`Capability::mount`] → [`Tagged`].

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
    pub fn new(hooks: Hooks, items: Items) -> Self {
        Self {
            hooks,
            items,
            _tag: PhantomData,
        }
    }

    pub fn empty_hooks(items: Items) -> Self
    where
        Hooks: Default,
    {
        Self::new(Hooks::default(), items)
    }
}

impl<Tag, Items> CapStore<Tag, HNil, Items> {
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
    pub fn add_hook<HTag, H>(
        self,
        hook: H,
    ) -> CapStore<Tag, HCons<Tagged<HTag, H>, Hooks>, Items> {
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
    pub fn add_item<ITag, I>(
        self,
        item: I,
    ) -> CapStore<Tag, Hooks, HCons<Tagged<ITag, I>, Items>> {
        CapStore {
            hooks: self.hooks,
            items: HCons {
                head: Tagged::new(item),
                tail: self.items,
            },
            _tag: PhantomData,
        }
    }

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
/// capabilities ([`crate::apps::AppsCapability`], [`crate::llm_tools::LlmToolsCapability`], …)
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

/// `ApplyHooks` for a local hook type that implements a registrar trait.
macro_rules! apply_registrar_hook {
    (
        capability: $cap:ident;
        trait: $trait:ident;
        method: $method:ident;
        field: $field:ident;
        proof: $proof:ty;
        tag: $tag:ty;
    ) => {
        impl<Plugin, H, Tail, Items, TailOut, Out, TailProof>
            $crate::capability::ApplyHooks<Items, (TailProof, $proof)>
            for ::frunk::HCons<$crate::tag::Tagged<Plugin, H>, Tail>
        where
            Items: $crate::capability::RegistrarItems,
            Tail: $crate::capability::ApplyHooks<Items, TailProof, Output = TailOut>,
            H: $trait<TailOut, Output = Out>,
        {
            type Output = Out;

            fn apply_hooks(self, items: Items) -> Self::Output {
                let items = self.tail.apply_hooks(items);
                H::$method(self.head.value, $cap { $field: items }).$field
            }
        }

        impl<Plugin, H, Tail, Items, TailOut, Out>
            $crate::capability::FoldRegistrarHooks<$tag, Items>
            for ::frunk::HCons<$crate::tag::Tagged<Plugin, H>, Tail>
        where
            Items: $crate::capability::RegistrarItems,
            Tail: $crate::capability::FoldRegistrarHooks<$tag, Items, Output = TailOut>,
            H: $trait<TailOut, Output = Out>,
        {
            type Output = Out;

            fn fold_registrar_hooks(self, items: Items) -> Self::Output {
                let items = self.tail.fold_registrar_hooks(items);
                H::$method(self.head.value, $cap { $field: items }).$field
            }
        }
    };
}

pub(crate) use apply_registrar_hook;

/// Declare item tags and a `Register*` impl for a marker-style capability.
///
/// Two item shapes are supported (add-order; last entry is HList head):
///
/// - `Tag => Value` — `add::<Tag, Value>()` → `Tagged<Tag, Wrapper<Value>>`
///   (templates)
/// - `Tag, Kind => Value` — `add::<Tag, Kind, Value>()` →
///   `Tagged<Tag, Wrapper<Kind, Value>>` (slots)
/// - empty `items: []` — identity `Register*` (hook queued, nothing added)
///
/// `$idx` names are call-site documentation matching index generics in
/// [`crate::plugin_routes::define_plugin_routes`]; frunk indices are inferred
/// from the concrete [`frunk::HList!`] `Output`.
///
/// ```ignore
/// define_register_items! {
///     plugin: UsersTag;
///     capability: TemplateCapability;
///     trait: RegisterTemplates;
///     method: register_templates;
///     wrapper: TemplateOf;
///     bounds: [Clone, ProvideRequestCaps, Send, Sync];
///     items: [
///         LoginIdx: UsersLoginPageTag => LoginPage,
///     ]
/// }
///
/// define_register_items! {
///     plugin: UsersTag;
///     capability: SlotCapability;
///     trait: RegisterSlots;
///     method: register_slots;
///     bounds: [];
///     items: [];
/// }
/// ```
#[macro_export]
macro_rules! define_register_items {
    // --- identity (no items) ---
    (
        plugin: $plugin:ty;
        capability: $cap:ident;
        trait: $trait:ident;
        method: $method:ident;
        bounds: [$($bound:path),* $(,)?];
        items: [];
        hook: $hook:ident;
    ) => {
        #[derive(Clone, Copy, Default)]
        pub struct $hook;

        impl<T: ::frunk::hlist::HList $(+ $bound)*> $trait<T> for $hook {
            type Output = T;

            fn $method(self, cap: $cap<T>) -> $cap<Self::Output> {
                cap
            }
        }
    };

    // --- two-param add (`Tag => Value`) ---
    (
        plugin: $plugin:ty;
        capability: $cap:ident;
        trait: $trait:ident;
        method: $method:ident;
        wrapper: $wrapper:ident;
        bounds: [$($bound:path),* $(,)?];
        hook: $hook:ident;
        items: [
            $($idx:ident : $tag:ident => $value:ty),+ $(,)?
        ]
    ) => {
        $(
            pub struct $tag;
        )+

        $crate::capability::define_register_items!(
            @impl2 $plugin;
            $cap;
            $trait;
            $method;
            $wrapper;
            $hook;
            [$($bound),*];
            [];
            [];
            $($tag => $value),+
        );
    };

    (
        @impl2 $plugin:ty;
        $cap:ident;
        $trait:ident;
        $method:ident;
        $wrapper:ident;
        $hook:ident;
        [$($bound:path),*];
        [$($rev_tag:ident => $rev_value:ty),*];
        [$($fwd_tag:ident => $fwd_value:ty),*];
        $tag:ident => $value:ty
        $(, $rest_tag:ident => $rest_value:ty)*
    ) => {
        $crate::capability::define_register_items!(
            @impl2 $plugin;
            $cap;
            $trait;
            $method;
            $wrapper;
            $hook;
            [$($bound),*];
            [$tag => $value $(, $rev_tag => $rev_value)*];
            [$($fwd_tag => $fwd_value,)* $tag => $value];
            $($rest_tag => $rest_value),*
        );
    };
    (
        @impl2 $plugin:ty;
        $cap:ident;
        $trait:ident;
        $method:ident;
        $wrapper:ident;
        $hook:ident;
        [$($bound:path),*];
        [$($rev_tag:ident => $rev_value:ty),+];
        [$($fwd_tag:ident => $fwd_value:ty),+];
    ) => {
        #[derive(Clone, Copy, Default)]
        pub struct $hook;

        #[allow(
            clippy::type_complexity,
            reason = "HList![…] of registered items plus prior plugins' T"
        )]
        impl<T: ::frunk::hlist::HList $(+ $bound)*> $trait<T> for $hook {
            type Output = ::frunk::HList![
                $($crate::tag::Tagged<$rev_tag, $wrapper<$rev_value>>),+,
                ...T
            ];

            fn $method(self, cap: $cap<T>) -> $cap<Self::Output> {
                cap
                    $(.add::<$fwd_tag, $fwd_value>())+
            }
        }
    };

    // --- three-param add (`Tag, Kind => Value`) ---
    (
        plugin: $plugin:ty;
        capability: $cap:ident;
        trait: $trait:ident;
        method: $method:ident;
        wrapper: $wrapper:ident;
        bounds: [$($bound:path),* $(,)?];
        hook: $hook:ident;
        items: [
            $($idx:ident : $tag:ident, $kind:ty => $value:ty),+ $(,)?
        ]
    ) => {
        $(
            pub struct $tag;
        )+

        $crate::capability::define_register_items!(
            @impl3 $plugin;
            $cap;
            $trait;
            $method;
            $wrapper;
            $hook;
            [$($bound),*];
            [];
            [];
            $($tag, $kind => $value),+
        );
    };

    (
        @impl3 $plugin:ty;
        $cap:ident;
        $trait:ident;
        $method:ident;
        $wrapper:ident;
        $hook:ident;
        [$($bound:path),*];
        [$($rev_tag:ident, $rev_kind:ty => $rev_value:ty),*];
        [$($fwd_tag:ident, $fwd_kind:ty => $fwd_value:ty),*];
        $tag:ident, $kind:ty => $value:ty
        $(, $rest_tag:ident, $rest_kind:ty => $rest_value:ty)*
    ) => {
        $crate::capability::define_register_items!(
            @impl3 $plugin;
            $cap;
            $trait;
            $method;
            $wrapper;
            $hook;
            [$($bound),*];
            [$tag, $kind => $value $(, $rev_tag, $rev_kind => $rev_value)*];
            [$($fwd_tag, $fwd_kind => $fwd_value,)* $tag, $kind => $value];
            $($rest_tag, $rest_kind => $rest_value),*
        );
    };
    (
        @impl3 $plugin:ty;
        $cap:ident;
        $trait:ident;
        $method:ident;
        $wrapper:ident;
        $hook:ident;
        [$($bound:path),*];
        [$($rev_tag:ident, $rev_kind:ty => $rev_value:ty),+];
        [$($fwd_tag:ident, $fwd_kind:ty => $fwd_value:ty),+];
    ) => {
        #[derive(Clone, Copy, Default)]
        pub struct $hook;

        #[allow(
            clippy::type_complexity,
            reason = "HList![…] of registered items plus prior plugins' T"
        )]
        impl<T: ::frunk::hlist::HList $(+ $bound)*> $trait<T> for $hook {
            type Output = ::frunk::HList![
                $($crate::tag::Tagged<$rev_tag, $wrapper<$rev_kind, $rev_value>>),+,
                ...T
            ];

            fn $method(self, cap: $cap<T>) -> $cap<Self::Output> {
                cap
                    $(.add::<$fwd_tag, $fwd_kind, $fwd_value>())+
            }
        }
    };
}

pub use crate::define_register_items;

/// Register a template hook that replaces tagged pages (and optionally prepends new ones).
#[macro_export]
macro_rules! define_replace_templates {
    (
        hook: $hook:ident;
        bounds: [$($bound:path),* $(,)?];
        replaces: [$($idx:ident : $rep_tag:ident => $rep_value:ty),* $(,)?];
        adds: [$($add_tag:ident => $add_value:ty),* $(,)?];
    ) => {
        $crate::define_replace_templates!(
            @go $hook;
            [$($bound),*];
            T;
            [];
            [];
            [];
            $($idx, $rep_tag, $rep_value),*;
            $($add_tag, $add_value),*
        );
    };

    (
        @go $hook:ident;
        [$($bound:path),*];
        $acc:ty;
        [$($where_acc:ty => $where_idx:ident, $where_tag:ident, $where_value:ty);*];
        [$($done_idx:ident),*];
        [$($done_rep_tag:ident, $done_rep_value:ty),*];
        $idx:ident, $rep_tag:ident, $rep_value:ty
        $(, $rest_idx:ident, $rest_tag:ident, $rest_value:ty)*;
        $($add_tag:ident, $add_value:ty),*
    ) => {
        $crate::define_replace_templates!(
            @go $hook;
            [$($bound),*];
            $crate::traits::get::ReplaceTemplateAtTag<
                $acc,
                $rep_tag,
                $crate::template::TemplateOf<$rep_value>,
                $idx
            >;
            [$($where_acc => $where_idx, $where_tag, $where_value;)* $acc => $idx, $rep_tag, $rep_value];
            [$($done_idx,)* $idx];
            [$($done_rep_tag, $done_rep_value,)* $rep_tag, $rep_value];
            $($rest_idx, $rest_tag, $rest_value),*;
            $($add_tag, $add_value),*
        );
    };

    (
        @go $hook:ident;
        [$($bound:path),*];
        $acc:ty;
        [$($where_acc:ty => $where_idx:ident, $where_tag:ident, $where_value:ty);*];
        [$($idx:ident),*];
        [$($rep_tag:ident, $rep_value:ty),*];
        ;
        $($add_tag:ident, $add_value:ty),*
    ) => {
        $crate::define_replace_templates!(
            @finish $hook;
            [$($bound),*];
            $acc;
            [$($where_acc => $where_idx, $where_tag, $where_value);*];
            [$($idx),*];
            [$($rep_tag, $rep_value),*];
            $($add_tag, $add_value),*
        );
    };

    (
        @finish $hook:ident;
        [$($bound:path),*];
        $acc:ty;
        [$($where_acc:ty => $where_idx:ident, $where_tag:ident, $where_value:ty);*];
        [$($idx:ident),*];
        [$($rep_tag:ident, $rep_value:ty),*];
    ) => {
        #[derive(Clone, Copy, Default)]
        pub struct $hook;

        impl<T: ::frunk::hlist::HList $(+ $bound)* $(, $idx)*> $crate::template::TemplateRegistrar<T> for $hook
        where
            $acc: ::frunk::hlist::HList,
            $( $where_acc:
                $crate::traits::get::IndexOfTemplateTag<
                    $where_tag,
                    $where_idx
                >, )*
            $( $where_acc:
                $crate::traits::replace::MapByTag<
                    $where_tag,
                    $crate::template::TemplateOf<$where_value>,
                    $where_idx
                >, )*
        {
            type Output = $acc;

            fn register_templates(
                self,
                cap: $crate::template::TemplateCapability<T>,
            ) -> $crate::template::TemplateCapability<Self::Output> {
                cap
                    $(.replace_template_tag::<
                        $rep_tag,
                        $crate::template::TemplateOf<$rep_value>,
                        _,
                    >(|_| $crate::template::TemplateOf::new()))*
            }
        }
    };

    (
        @finish $hook:ident;
        [$($bound:path),*];
        $acc:ty;
        [$($where_acc:ty => $where_idx:ident, $where_tag:ident, $where_value:ty);*];
        [$($idx:ident),*];
        [$($rep_tag:ident, $rep_value:ty),*];
        $($add_tag:ident, $add_value:ty),+
    ) => {
        $crate::define_replace_templates!(
            @collect_adds $hook;
            [$($bound),*];
            $acc;
            [$($where_acc => $where_idx, $where_tag, $where_value);*];
            [$($idx),*];
            [$($rep_tag, $rep_value),*];
            [];
            [];
            $($add_tag, $add_value),+
        );
    };

    (
        @collect_adds $hook:ident;
        [$($bound:path),*];
        $acc:ty;
        [$($where_acc:ty => $where_idx:ident, $where_tag:ident, $where_value:ty);*];
        [$($idx:ident),*];
        [$($rep_tag:ident, $rep_value:ty),*];
        [$($rev_tag:ident, $rev_value:ty),*];
        [$($fwd_tag:ident, $fwd_value:ty),*];
        $add_tag:ident, $add_value:ty
        $(, $rest_tag:ident, $rest_value:ty)*
    ) => {
        $crate::define_replace_templates!(
            @collect_adds $hook;
            [$($bound),*];
            $acc;
            [$($where_acc => $where_idx, $where_tag, $where_value);*];
            [$($idx),*];
            [$($rep_tag, $rep_value),*];
            [$add_tag, $add_value $(, $rev_tag, $rev_value)*];
            [$($fwd_tag, $fwd_value,)* $add_tag, $add_value];
            $($rest_tag, $rest_value),*
        );
    };

    (
        @collect_adds $hook:ident;
        [$($bound:path),*];
        $acc:ty;
        [$($where_acc:ty => $where_idx:ident, $where_tag:ident, $where_value:ty);*];
        [$($idx:ident),*];
        [$($rep_tag:ident, $rep_value:ty),*];
        [$($rev_tag:ident, $rev_value:ty),+];
        [$($fwd_tag:ident, $fwd_value:ty),+];
    ) => {
        #[derive(Clone, Copy, Default)]
        pub struct $hook;

        #[allow(
            clippy::type_complexity,
            reason = "HList of prepended templates plus replaced chain"
        )]
        impl<T: ::frunk::hlist::HList $(+ $bound)* $(, $idx)*> $crate::template::TemplateRegistrar<T> for $hook
        where
            $acc: ::frunk::hlist::HList,
            $( $where_acc:
                $crate::traits::get::IndexOfTemplateTag<
                    $where_tag,
                    $where_idx
                >, )*
            $( $where_acc:
                $crate::traits::replace::MapByTag<
                    $where_tag,
                    $crate::template::TemplateOf<$where_value>,
                    $where_idx
                >, )*
        {
            type Output = ::frunk::HList![
                $($crate::tag::Tagged<
                    $rev_tag,
                    $crate::template::TemplateOf<$rev_value>
                >),+,
                ...$acc
            ];

            fn register_templates(
                self,
                cap: $crate::template::TemplateCapability<T>,
            ) -> $crate::template::TemplateCapability<Self::Output> {
                cap
                    $(.replace_template_tag::<
                        $rep_tag,
                        $crate::template::TemplateOf<$rep_value>,
                        _,
                    >(|_| $crate::template::TemplateOf::new()))*
                    $(.add::<$fwd_tag, $fwd_value>())+
            }
        }
    };
}

pub use crate::define_replace_templates;



/// `CapStore<Tag, HNil, Value>` that mounts as [`Tagged::new`](crate::tag::Tagged::new)(`items`).
///
/// ```ignore
/// define_passthrough_cap!(UsersStateCap, UsersTag, UsersState);
/// ```
#[macro_export]
macro_rules! define_passthrough_cap_impl {
    ($cap:ident, $tag:ty, $value:ty) => {
        pub type $cap = $crate::capability::CapStore<$tag, ::frunk::HNil, $value>;

        impl $crate::capability::PassthroughCapTag for $tag {}
    };
}

#[macro_export]
macro_rules! define_passthrough_cap {
    ($($tt:tt)*) => {
        $crate::define_passthrough_cap_impl! { $($tt)* }
    };
}

pub use crate::define_passthrough_cap;

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
