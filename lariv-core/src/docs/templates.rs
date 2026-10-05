//! Maud page templates and template registration.
//!
//! # Page templates (`templates.rs`)
//!
//! In Lariv, a "page" is a Rust struct implementing [`RenderTemplate`](crate::template::RenderTemplate).
//! Pages return [`maud::Markup`] — HTML built at compile time, not from external template files
//! (though the website plugin also supports Minijinja for CMS content).
//!
//! # Basic page
//!
//! ```ignore
//! use maud::{Markup, html};
//! use lariv_rs::{
//!     components::ShellChrome,
//!     template::RenderTemplate,
//! };
//!
//! pub struct GreetingPage {
//!     pub name: String,
//! }
//!
//! impl RenderTemplate for GreetingPage {
//!     fn render(&self, chrome: &ShellChrome) -> Markup {
//!         html! {
//!             div class="container mx-auto" {
//!                 h1 class="text-2xl font-bold" { "Hello, " (self.name) "!" }
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! # HTMX partials
//!
//! For pages that support HTMX app-layout navigation, also implement
//! [`RenderAppPane`](crate::template::RenderAppPane):
//!
//! ```ignore
//! impl RenderAppPane for GreetingPage {
//!     fn render_pane(&self) -> crate::components::AppLayoutHtml {
//!         crate::components::layout_sidebar(crate::components::LayoutSidebar {
//!             sidebar: my_sidebar(),
//!             breadcrumbs: crate::components::breadcrumbs(&[crate::components::Crumb {
//!                 label: "My App",
//!                 href: None,
//!             }]),
//!             content: self.page_body(),
//!         })
//!     }
//!
//!     fn render_main(&self) -> crate::components::MainContentHtml {
//!         crate::components::layout_main(crate::components::LayoutMain {
//!             breadcrumbs: maud::Markup::default(),
//!             content: self.page_body(),
//!         })
//!     }
//! }
//! ```
//!
//! Handlers call [`html_built_page_or_app_layout`](crate::web::html_built_page_or_app_layout)
//! to pick full document vs partial based on the HTMX request.
//!
//! # Registering templates
//!
//! Use [`define_register_items!`](crate::capability::define_register_items) in `templates.rs`:
//!
//! ```ignore
//! use lariv_rs::capability::define_register_items;
//! use lariv_rs::template::{TemplateCapability, TemplateOf, TemplateRegistrar};
//!
//! pub struct GreetingPageTag;
//!
//! define_register_items! {
//!     plugin: MyPluginTag;
//!     capability: TemplateCapability;
//!     trait: TemplateRegistrar;
//!     method: register_templates;
//!     wrapper: TemplateOf;
//!     bounds: [Clone];
//!     hook: Hook;
//!     items: [
//!         GreetingIdx: GreetingPageTag => GreetingPage,
//!     ]
//! }
//! ```
//!
//! Add `templates(templates::Hook)` to [`define_plugin_install!`](crate::plugin_install::define_plugin_install).
//!
//! # Replacing another plugin's template
//!
//! Addon plugins can override pages registered by another plugin using
//! [`define_replace_templates!`](crate::capability::define_replace_templates) with a compile-time
//! index (see the `signup` plugin for login page patches).
//!
//! # Shell chrome and slots
//!
//! Full pages wrap content in navigation chrome from [`ShellChrome`](crate::components::ShellChrome).
//! Plugins contribute topbar/head fragments via `define_register_items!` on
//! [`SlotCapability`](crate::components::slots::SlotCapability) — see the dashboard plugin
//! for topbar buttons and user dropdown examples.
//!
//! # UI building blocks
//!
//! Compose pages from [`crate::components`] builders — fields, inputs, tables, forms, and layout
//! containers. See [`super::components`].
