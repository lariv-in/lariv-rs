//! Compile-time template registry for page types.
//!
//! Plugins register [`TemplateOf`] markers on a capability HList. Pages implement
//! [`RenderTemplate`] (full document with chrome) and optionally [`RenderAppPane`]
//! (HTMX partials).
//!
//! # Routes
//!
//! Call [`with_templates`] on the app builder, then use [`TemplateRegistrar`] hooks to
//! append page markers. Resolve hooks with [`TemplateCap::resolve_hooks`] before mount.
//!
//! # Use cases
//!
//! - Declare which page types a plugin exposes for compile-time lookup by tag.
//! - Replace or extend registered templates during plugin install.
//! - Render Maud pages with shared slot chrome from handlers or view layers.
//!
//! # Examples
//!
//! ```rust ignore
//! // Plugin install:
//! app.templates()
//!     .register(|cap| cap.add::<UserListTag, UserListPage>())
//!     .register(|cap| cap.add::<UserEditTag, UserEditPage>());
//! ```

use std::marker::PhantomData;

use frunk::{HCons, HNil, hlist::HList};
use maud::Markup;

use crate::{
    app::App,
    capability::{ApplyHooks, CapStore, Capability, FoldRegistrarHooks, apply_registrar_hook, mount_with_hooks},
    tag::Tagged,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::{GetByTag, IndexOfTemplateTag},
        replace::MapByTag,
    },
};

/// Capability tag for the compile-time template registry HList.
pub struct TemplateTag;

/// Render a page type to Maud [`Markup`] with document chrome.
///
/// # Use cases
///
/// - Full-page HTML responses with navigation shell.
/// - Base trait for [`RenderAppPane`] partial rendering.
pub trait RenderTemplate {
    fn render(&self, chrome: &crate::components::ShellChrome) -> Markup;
}

/// Fine-grained HTMX fragments without document chrome.
///
/// Default [`render_main`](Self::render_main) delegates to [`render_pane`](Self::render_pane);
/// override when `<main id="main-content">` differs from `#app-layout`.
pub trait RenderAppPane {
    /// Markup for `#app-layout` swaps (boosted nav, form POST into pane).
    fn render_pane(&self) -> Markup;

    /// Markup for `<main id="main-content">` swaps.
    fn render_main(&self) -> Markup {
        self.render_pane()
    }
}

/// Type-level marker registering page type `T` on the template HList.
///
/// Carries no runtime data; presence on the HList enables [`GetByTag`] lookup.
pub struct TemplateOf<T> {
    _page: PhantomData<fn() -> T>,
}

impl<T> Clone for TemplateOf<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for TemplateOf<T> {}

impl<T> Default for TemplateOf<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TemplateOf<T> {
    /// Construct a zero-sized marker for page type `T`.
    pub const fn new() -> Self {
        Self {
            _page: PhantomData,
        }
    }
}

/// Plugin hook for appending template markers onto a [`TemplateCapability`].
pub trait TemplateRegistrar<T>: Sized {
    type Output;
    fn register_templates(self, cap: TemplateCapability<T>) -> TemplateCapability<Self::Output>;
}
/// Mounted template capability: HList of [`TemplateOf`] markers keyed by tag.
#[derive(Clone)]
pub struct TemplateCapability<Templates> {
    pub templates: Templates,
}

impl TemplateCapability<HNil> {
    pub fn new() -> Self {
        Self { templates: HNil }
    }
}

impl Default for TemplateCapability<HNil> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Templates> TemplateCapability<Templates> {
    /// Prepend a template marker for page type `T` under compile-time tag `Tag`.
    pub fn add<Tag, T>(self) -> TemplateCapability<HCons<Tagged<Tag, TemplateOf<T>>, Templates>>
    where
        Templates: HList,
        T: RenderTemplate + 'static,
    {
        TemplateCapability {
            templates: HCons {
                head: Tagged::new(TemplateOf::new()),
                tail: self.templates,
            },
        }
    }

    pub fn get_template<Tag, Index>(&self) -> &<Templates as GetByTag<Tag, Index>>::Value
    where
        Templates: GetByTag<Tag, Index>,
    {
        self.templates.get_by_tag()
    }

    pub fn replace_template<Tag, Index, NewValue>(
        self,
        f: impl FnOnce(<Templates as MapByTag<Tag, NewValue, Index>>::OldValue) -> NewValue,
    ) -> TemplateCapability<<Templates as MapByTag<Tag, NewValue, Index>>::Output>
    where
        Templates: MapByTag<Tag, NewValue, Index>,
    {
        TemplateCapability {
            templates: self.templates.map_by_tag(f),
        }
    }

    /// Like [`replace_template`](Self::replace_template) with the frunk index inferred from [`IndexOfTemplateTag`].
    pub fn replace_template_tag<Tag, NewValue, Index>(
        self,
        f: impl FnOnce(<Templates as MapByTag<Tag, NewValue, Index>>::OldValue) -> NewValue,
    ) -> TemplateCapability<<Templates as MapByTag<Tag, NewValue, Index>>::Output>
    where
        Templates: IndexOfTemplateTag<Tag, Index>,
        Templates: MapByTag<Tag, NewValue, Index>,
    {
        self.replace_template::<Tag, Index, NewValue>(f)
    }
}

/// Builder-phase template capability (`hooks` queues [`TemplateRegistrar`] plugins).
pub type TemplateCap<Hooks, Items> = CapStore<TemplateTag, Hooks, Items>;

impl<Hooks, Items> TemplateCap<Hooks, Items> {
    /// Apply deferred register hooks and clear the hook list.
    pub fn resolve_hooks(self) -> TemplateCap<HNil, <Hooks as FoldRegistrarHooks<TemplateTag, Items>>::Output>
    where
        Hooks: FoldRegistrarHooks<TemplateTag, Items>,
    {
        CapStore::with_items(self.hooks.fold_registrar_hooks(self.items))
    }
}

apply_registrar_hook! {
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    field: templates;
    proof: crate::capability::TemplateHookProof;
    tag: TemplateTag;
}

impl<Hooks, Items> Capability for TemplateCap<Hooks, Items>
where
    Hooks: ApplyHooks<Items>,
{
    type Value = TemplateCapability<Hooks::Output>;
    type Output = Tagged<TemplateTag, TemplateCapability<Hooks::Output>>;
    type Hooks = Hooks;
    type Items = Items;

    fn mount(self) -> Self::Output {
        mount_with_hooks(self, |items| TemplateCapability { templates: items })
    }
}

/// Add an empty template registry to `app` (call before plugins register pages).
///
/// # Examples
///
/// ```rust
/// # use lariv_rs::{app::App, template::with_templates};
/// let app = with_templates(App::new());
/// ```
pub fn with_templates<L, Proof>(app: App<L>) -> App<HCons<TemplateCap<HNil, HNil>, L>>
where
    L: HList + CapTagAbsent<TemplateTag, Proof>,
{
    app.add_capability(CapStore::with_items(HNil))
}
