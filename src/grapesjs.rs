//! GrapesJS registries capability — plugins register blocks, components, traits, and themes.
//!
//! Mounted as [`Arc<GrapesJsCapability>`] so request extensions share large HTML/JSON catalogs
//! without deep copies.
//!
//! # Lifecycle
//!
//! 1. Attach via [`with_grapesjs`].
//! 2. Plugins implement [`GrapesJsRegistrar`] to mutate the capability in place.
//! 3. At mount, hooks fold → [`Arc<GrapesJsCapability>`] on the app HList.
//! 4. Builder UI reads [`GrapesJsCapability::blocks_json`] etc.; published pages use themes.
//!
//! # Core types
//!
//! - [`GrapesJsTag`] — capability tag
//! - [`GrapesJsBlock`] — BlockManager entry (HTML content, category, media)
//! - [`GrapesJsComponent`] — DomComponents type definition
//! - [`GrapesJsTrait`] — Traits panel custom type
//! - [`GrapesJsTheme`] — named CSS theme for builder and published pages
//! - [`GrapesJsCapability`] — mounted four-registry catalog
//! - [`GrapesJsCap`] — builder-phase [`CapStore`]
//! - [`GrapesJsRegistrar`] — plugin hook trait
//!
//! # Stack safety
//!
//! Registrar hooks **must** mutate via `&mut GrapesJsCapability` (not chain by-value returns).
//! Large block/theme catalogs can overflow the tokio worker stack if cloned during mount.
//!
//! # Examples
//!
//! ```rust ignore
//! impl GrapesJsRegistrar for RegisterGrapesJsHook {
//!     fn register_grapesjs(self, gjs: &mut GrapesJsCapability) {
//!         gjs.register_block("hero", GrapesJsBlock::html("Hero", "Layout", "<section>...</section>"));
//!     }
//! }
//!
//! let app = with_grapesjs(app);
//! ```

use std::sync::Arc;

use frunk::{HCons, HNil, hlist::HList};
use serde::Serialize;
use serde_json::{Value, json};

use crate::{
    app::App,
    capability::{ApplyHooks, CapStore, Capability, mount_with_hooks},
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};

/// Capability tag for GrapesJS registries.
pub struct GrapesJsTag;

/// BlockManager.add props. Registry key is the block id.
#[derive(Clone, Debug, Serialize)]
pub struct GrapesJsBlock {
    pub label: String,
    pub content: Value,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub media: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Value>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub activate: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub select: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub disable: bool,
    #[serde(rename = "onClick", skip_serializing_if = "Option::is_none")]
    pub on_click: Option<Value>,
}

impl GrapesJsBlock {
    /// Convenience constructor for an HTML block in a named category.
    pub fn html(
        label: impl Into<String>,
        category: impl Into<String>,
        html: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            content: Value::String(html.into()),
            media: String::new(),
            category: Some(Value::String(category.into())),
            attributes: None,
            activate: false,
            select: false,
            disable: false,
            on_click: None,
        }
    }
}

/// DomComponents.addType props.
#[derive(Clone, Debug, Serialize, Default)]
pub struct GrapesJsComponent {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub extend: String,
    #[serde(rename = "isComponent", skip_serializing_if = "Option::is_none")]
    pub is_component: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<Value>,
}

/// Traits.addType props.
#[derive(Clone, Debug, Serialize, Default)]
pub struct GrapesJsTrait {
    #[serde(rename = "noLabel", skip_serializing_if = "std::ops::Not::not")]
    pub no_label: bool,
    #[serde(rename = "eventCapture", skip_serializing_if = "Vec::is_empty")]
    pub event_capture: Vec<String>,
    #[serde(rename = "templateInput", skip_serializing_if = "Option::is_none")]
    pub template_input: Option<Value>,
    #[serde(rename = "createInput", skip_serializing_if = "Option::is_none")]
    pub create_input: Option<Value>,
    #[serde(rename = "createLabel", skip_serializing_if = "Option::is_none")]
    pub create_label: Option<Value>,
    #[serde(rename = "onEvent", skip_serializing_if = "Option::is_none")]
    pub on_event: Option<Value>,
    #[serde(rename = "onUpdate", skip_serializing_if = "Option::is_none")]
    pub on_update: Option<Value>,
}

/// Named CSS theme for the builder and published pages.
#[derive(Clone, Debug, Serialize, Default)]
pub struct GrapesJsTheme {
    pub label: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub css: String,
    /// When set (e.g. `text/tailwindcss`), applied as the `<style>` element `type` attribute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub css_type: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stylesheets: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scripts: Vec<String>,
}

/// Plugin hook for appending GrapesJS entries onto a [`GrapesJsCapability`].
///
/// Implementations must mutate in place (not chain by-value `register_*` returns);
/// catalogs with large HTML/JSON will otherwise overflow the tokio worker stack at mount.
pub trait GrapesJsRegistrar {
    fn register_grapesjs(self, grapesjs: &mut GrapesJsCapability);
}

/// Runtime GrapesJS registries (mounted value published into request extensions).
#[derive(Clone, Debug, Default)]
pub struct GrapesJsCapability {
    blocks: Vec<(String, GrapesJsBlock)>,
    components: Vec<(String, GrapesJsComponent)>,
    traits: Vec<(String, GrapesJsTrait)>,
    themes: Vec<(String, GrapesJsTheme)>,
}

impl GrapesJsCapability {
    /// Empty registries (starting point for [`GrapesJsRegistrar`] hooks).
    pub fn new() -> Self {
        Self::default()
    }

    fn upsert<T>(entries: &mut Vec<(String, T)>, id: String, value: T) {
        if let Some(existing) = entries.iter_mut().find(|(k, _)| *k == id) {
            existing.1 = value;
        } else {
            entries.push((id, value));
        }
    }

    /// Register (or replace) a block. Prefer `&mut self` over by-value chaining so
    /// large catalogs do not blow the stack during mount.
    pub fn register_block(&mut self, id: impl Into<String>, block: GrapesJsBlock) -> &mut Self {
        Self::upsert(&mut self.blocks, id.into(), block);
        self
    }

    pub fn register_component(
        &mut self,
        id: impl Into<String>,
        component: GrapesJsComponent,
    ) -> &mut Self {
        Self::upsert(&mut self.components, id.into(), component);
        self
    }

    /// Register (or replace) a custom trait type by id.
    pub fn register_trait(&mut self, id: impl Into<String>, trait_: GrapesJsTrait) -> &mut Self {
        Self::upsert(&mut self.traits, id.into(), trait_);
        self
    }

    /// Register (or replace) a named CSS theme by id.
    pub fn register_theme(&mut self, id: impl Into<String>, theme: GrapesJsTheme) -> &mut Self {
        Self::upsert(&mut self.themes, id.into(), theme);
        self
    }

    /// All registered blocks as `(id, block)` pairs.
    pub fn blocks(&self) -> &[(String, GrapesJsBlock)] {
        &self.blocks
    }

    /// All registered component types as `(id, component)` pairs.
    pub fn components(&self) -> &[(String, GrapesJsComponent)] {
        &self.components
    }

    /// All registered trait types as `(id, trait)` pairs.
    pub fn traits(&self) -> &[(String, GrapesJsTrait)] {
        &self.traits
    }

    /// All registered themes as `(id, theme)` pairs.
    pub fn themes(&self) -> &[(String, GrapesJsTheme)] {
        &self.themes
    }

    /// Look up a theme by id.
    pub fn theme(&self, id: &str) -> Option<&GrapesJsTheme> {
        self.themes.iter().find(|(k, _)| k == id).map(|(_, t)| t)
    }

    /// Builder JSON payload: `[{id, ...fields}, ...]`.
    pub fn blocks_json(&self) -> Value {
        Value::Array(
            self.blocks
                .iter()
                .map(|(id, b)| {
                    let mut v = serde_json::to_value(b).unwrap_or(json!({}));
                    if let Value::Object(ref mut map) = v {
                        map.insert("id".into(), Value::String(id.clone()));
                    }
                    v
                })
                .collect(),
        )
    }

    /// Builder JSON payload for components: `[{id, ...fields}, ...]`.
    pub fn components_json(&self) -> Value {
        Value::Array(
            self.components
                .iter()
                .map(|(id, c)| {
                    let mut v = serde_json::to_value(c).unwrap_or(json!({}));
                    if let Value::Object(ref mut map) = v {
                        map.insert("id".into(), Value::String(id.clone()));
                    }
                    v
                })
                .collect(),
        )
    }

    /// Builder JSON payload for traits: `[{id, ...fields}, ...]`.
    pub fn traits_json(&self) -> Value {
        Value::Array(
            self.traits
                .iter()
                .map(|(id, t)| {
                    let mut v = serde_json::to_value(t).unwrap_or(json!({}));
                    if let Value::Object(ref mut map) = v {
                        map.insert("id".into(), Value::String(id.clone()));
                    }
                    v
                })
                .collect(),
        )
    }

    /// Builder JSON payload for themes: `[{id, ...fields}, ...]`.
    pub fn themes_json(&self) -> Value {
        Value::Array(
            self.themes
                .iter()
                .map(|(id, t)| {
                    let mut v = serde_json::to_value(t).unwrap_or(json!({}));
                    if let Value::Object(ref mut map) = v {
                        map.insert("id".into(), Value::String(id.clone()));
                    }
                    v
                })
                .collect(),
        )
    }
}

/// Builder-phase GrapesJS capability.
pub type GrapesJsCap<Hooks> = CapStore<GrapesJsTag, Hooks, GrapesJsCapability>;

impl<Hooks> GrapesJsCap<Hooks> {
    /// Eagerly fold registrar hooks into items (testing / pre-mount inspection).
    pub fn resolve_hooks<Proof>(self) -> GrapesJsCap<HNil>
    where
        Hooks: ApplyHooks<GrapesJsCapability, Proof, Output = GrapesJsCapability>,
    {
        CapStore::with_items(self.hooks.apply_hooks(self.items))
    }
}

impl<Plugin, H, Tail, TailProof> ApplyHooks<GrapesJsCapability, (TailProof, ())>
    for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: ApplyHooks<GrapesJsCapability, TailProof, Output = GrapesJsCapability>,
    H: GrapesJsRegistrar,
{
    type Output = GrapesJsCapability;

    fn apply_hooks(self, items: GrapesJsCapability) -> Self::Output {
        let mut items = self.tail.apply_hooks(items);
        self.head.value.register_grapesjs(&mut items);
        items
    }
}

impl<Hooks> Capability for GrapesJsCap<Hooks>
where
    Hooks: ApplyHooks<GrapesJsCapability, (), Output = GrapesJsCapability>,
{
    /// Shared so request-extension clones do not deep-copy block/theme HTML.
    type Value = Arc<GrapesJsCapability>;
    type Output = Tagged<GrapesJsTag, Arc<GrapesJsCapability>>;
    type Hooks = Hooks;
    type Items = GrapesJsCapability;

    fn mount(self) -> Self::Output {
        mount_with_hooks(self, Arc::new)
    }
}

/// Attach an empty GrapesJS registries capability to the app builder.
pub fn with_grapesjs<L, Proof>(app: App<L>) -> App<HCons<GrapesJsCap<HNil>, L>>
where
    L: HList + CapTagAbsent<GrapesJsTag, Proof>,
{
    app.add_capability(CapStore::with_items(GrapesJsCapability::new()))
}
