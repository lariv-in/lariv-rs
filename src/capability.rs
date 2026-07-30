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

/// `ApplyHooks` for a deferred `Register*` hook that wraps items in a capability,
/// calls `register_*`, and unwraps the same field.
///
/// ```ignore
/// apply_register_hook! {
///     hook: RegisterTemplatesHook;
///     capability: TemplateCapability;
///     trait: RegisterTemplates;
///     method: register_templates;
///     field: templates;
/// }
/// ```
macro_rules! apply_register_hook {
    (
        hook: $hook:ident;
        capability: $cap:ident;
        trait: $trait:ident;
        method: $method:ident;
        field: $field:ident;
    ) => {
        impl<Plugin, Tail, Items, TailOut, Out, TailProof, Proof>
            $crate::capability::ApplyHooks<Items, (TailProof, Proof)>
            for ::frunk::HCons<$crate::tag::Tagged<Plugin, $hook<Plugin>>, Tail>
        where
            Tail: $crate::capability::ApplyHooks<Items, TailProof, Output = TailOut>,
            $cap<TailOut>: $trait<Plugin, Proof, Output = $cap<Out>>,
        {
            type Output = Out;

            fn apply_hooks(self, items: Items) -> Self::Output {
                let items = self.tail.apply_hooks(items);
                $trait::<Plugin, Proof>::$method($cap {
                    $field: items,
                })
                .$field
            }
        }
    };
}

pub(crate) use apply_register_hook;

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
macro_rules! define_register_items {
    // --- identity (no items) ---
    (
        plugin: $plugin:ty;
        capability: $cap:ident;
        trait: $trait:ident;
        method: $method:ident;
        bounds: [$($bound:path),* $(,)?];
        items: [];
    ) => {
        impl<T: ::frunk::hlist::HList $(+ $bound)*> $trait<$plugin> for $cap<T> {
            type Output = $cap<T>;

            fn $method(self) -> Self::Output {
                self
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
        [$($bound:path),*];
        [$($rev_tag:ident => $rev_value:ty),+];
        [$($fwd_tag:ident => $fwd_value:ty),+];
    ) => {
        #[allow(
            clippy::type_complexity,
            reason = "HList![…] of registered items plus prior plugins' T"
        )]
        impl<T: ::frunk::hlist::HList $(+ $bound)*> $trait<$plugin> for $cap<T> {
            type Output = $cap<
                ::frunk::HList![
                    $($crate::tag::Tagged<$rev_tag, $wrapper<$rev_value>>,)+
                    ...T
                ],
            >;

            fn $method(self) -> Self::Output {
                self
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
        [$($bound:path),*];
        [$($rev_tag:ident, $rev_kind:ty => $rev_value:ty),+];
        [$($fwd_tag:ident, $fwd_kind:ty => $fwd_value:ty),+];
    ) => {
        #[allow(
            clippy::type_complexity,
            reason = "HList![…] of registered items plus prior plugins' T"
        )]
        impl<T: ::frunk::hlist::HList $(+ $bound)*> $trait<$plugin> for $cap<T> {
            type Output = $cap<
                ::frunk::HList![
                    $($crate::tag::Tagged<$rev_tag, $wrapper<$rev_kind, $rev_value>>,)+
                    ...T
                ],
            >;

            fn $method(self) -> Self::Output {
                self
                    $(.add::<$fwd_tag, $fwd_kind, $fwd_value>())+
            }
        }
    };
}

pub(crate) use define_register_items;

/// `CapStore<Tag, HNil, Value>` that mounts as [`Tagged::new`](crate::tag::Tagged::new)(`items`).
///
/// ```ignore
/// define_passthrough_cap!(UsersStateCap, UsersTag, UsersState);
/// ```
macro_rules! define_passthrough_cap {
    ($cap:ident, $tag:ty, $value:ty) => {
        pub type $cap = $crate::capability::CapStore<$tag, ::frunk::HNil, $value>;

        impl $crate::capability::Capability for $cap {
            type Value = $value;
            type Output = $crate::tag::Tagged<$tag, $value>;
            type Hooks = ::frunk::HNil;
            type Items = $value;

            fn mount(self) -> Self::Output {
                $crate::tag::Tagged::new(self.items)
            }
        }
    };
}

pub(crate) use define_passthrough_cap;

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
