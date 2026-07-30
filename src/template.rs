use std::marker::PhantomData;

use frunk::{HCons, HNil, hlist::HList};
use maud::Markup;

use crate::{
    app::App,
    capability::{ApplyHooks, CapStore, Capability, apply_register_hook, mount_with_hooks},
    hooks::zst_hook,
    tag::Tagged,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::GetByTag,
        replace::MapByTag,
    },
};

/// Capability tag for the template registry.
pub struct TemplateTag;

/// Render a page type to Maud [`Markup`].
pub trait RenderTemplate {
    fn render(&self, chrome: &crate::components::ShellChrome) -> Markup;
}

/// Fine-grained fragments for HTMX (no document chrome).
pub trait RenderAppPane {
    fn render_pane(&self) -> Markup;

    fn render_main(&self) -> Markup {
        self.render_pane()
    }
}

/// Type marker registering a page type `T` on the template HList.
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
    pub const fn new() -> Self {
        Self {
            _page: PhantomData,
        }
    }
}

zst_hook!(
    /// Deferred plugin hook: register templates at [`Capability::mount`] time.
    RegisterTemplatesHook
);

/// Mounted template registry (published into request extensions).
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
}

/// Builder-phase template capability.
pub type TemplateCap<Hooks, Items> = CapStore<TemplateTag, Hooks, Items>;

impl<Hooks, Items> TemplateCap<Hooks, Items> {
    /// Apply deferred register hooks and clear the hook list.
    pub fn resolve_hooks<Proof>(self) -> TemplateCap<HNil, <Hooks as ApplyHooks<Items, Proof>>::Output>
    where
        Hooks: ApplyHooks<Items, Proof>,
    {
        CapStore::with_items(self.hooks.apply_hooks(self.items))
    }
}

apply_register_hook! {
    hook: RegisterTemplatesHook;
    capability: TemplateCapability;
    trait: RegisterTemplates;
    method: register_templates;
    field: templates;
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

/// Plugin hook for appending template markers onto a [`TemplateCapability`].
pub trait RegisterTemplates<Plugin, Proof = ()>: Sized {
    type Output;
    fn register_templates(self) -> Self::Output;
}

pub fn with_templates<L, Proof>(app: App<L>) -> App<HCons<TemplateCap<HNil, HNil>, L>>
where
    L: HList + CapTagAbsent<TemplateTag, Proof>,
{
    app.add_capability(CapStore::with_items(HNil))
}
