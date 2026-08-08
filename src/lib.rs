#![feature(impl_trait_in_assoc_type)]
#![recursion_limit = "512"]

//! Lariv application kernel — configuration, HTTP server wiring, and aggregated plugin registries.
//!
//! **lariv-rs** is a compile-time plugin web application framework built on **Axum**,
//! **SeaORM**, **Maud**, and **HTMX 4**.
//!
//! # Architecture
//!
//! The app lifecycle has three phases:
//!
//! 1. **Builder** — [`App`](app::App) holds an HList of capability stores (hooks + items).
//!    Plugins call [`define_plugin_install!`](plugin_install::define_plugin_install) to register
//!    deferred hooks for routes, templates, migrations, CLI commands, etc.
//! 2. **Mount** — [`App::mount`](app::App::mount) resolves hooks, folds capabilities to
//!    [`Tagged`](tag::Tagged) outputs, and produces a [`MountedApp`](app::MountedApp).
//! 3. **Runtime** — Axum serves HTTP; handlers extract mounted state via [`Cap`](http::Cap).
//!
//! # Quickstart
//!
//! ```ignore
//! use lariv_rs::app::App;
//! use lariv_rs::plugins::{dashboard, users};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let app = App::new_web_app();
//!     let app = users::install(app);
//!     let app = dashboard::install(app);
//!     let app = app.load_config("config.toml").await?;
//!     let mounted = app.mount();
//!     mounted.run_migrations().await?;
//!     mounted.run_seeds().await?;
//!     mounted.run().await
//! }
//! ```
//!
//! See [`app`] for lifecycle details and [`plugins`] for bundled plugins.
//!
//! # Beginner guides
//!
//! Step-by-step tutorials for new plugin authors live in the [`docs`] module:
//!
//! - [`docs::quickstart`] — Hello World plugin tutorial
//! - [`docs`] — project layout and guide index
//!
//! # Core modules
//!
//! | Module | Role |
//! |--------|------|
//! | [`app`] | Builder and mounted app lifecycle |
//! | [`capability`] | Capability stores, hooks, mount folding |
//! | [`tag`] | Type-level tagging via [`tag::Tagged`] |
//! | [`traits`] | HList lookup, add, replace, remove |
//! | [`http`] | Route registry and Axum router |
//! | [`template`] | Maud page template registry |
//! | [`layers`] | Compile-time view middleware stacks |
//! | [`components`] | Maud UI builders (fields, tables, shells) |
//! | [`web`] | HTMX-aware page rendering helpers |
//! | [`html_form`] | Form macro and widget traits |
//! | [`config`] | TOML configuration loading |
//! | [`db`] | SeaORM connection capability |
//! | [`migration`] | Composite SeaORM migrator |
//! | [`command`] | CLI command registration |
//! | [`hooks`] | State attachment and seed hooks |
//! | [`apps`] | Dashboard app tile catalog |
//! | [`export`] | XLSX export table catalog |
//! | [`llm_tools`] | Gemini function-calling tools |
//! | [`rune_env`] | Rune script native bindings |
//! | [`grapesjs`] | Website builder block/component registries |
//! | [`genai`] | Gemini HTTP client |
//! | [`views`] | Named view registry (PWA offline page) |
//!
//! # Plugin authoring
//!
//! | Module | Role |
//! |--------|------|
//! | [`plugin_install`] | [`define_plugin_install!`] macro |
//! | [`plugin_routes`] | [`define_plugin_routes!`] DSL reference |
//! | [`define_plugin_routes`] | Proc-macro re-export |
//!
//! # Bundled plugins
//!
//! | Plugin | Module | Purpose |
//! |--------|--------|---------|
//! | Users & auth | [`plugins::users`] | JWT/scrypt auth, roles, user CRUD |
//! | Dashboard | [`plugins::dashboard`] | Apps launchpad and home redirects |
//! | Blog | [`plugins::blog`] | Articles and hierarchical tags |
//! | Filesystem | [`plugins::filesystem`] | DB-backed virtual filesystem |
//! | Website | [`plugins::website`] | DB routes, Minijinja pages, GrapesJS builder |
//! | LLM assistant | [`plugins::llm_assistant`] | Gemini chat, skills, WebSocket |
//! | OTP recovery | [`plugins::otp`] | SMS/email one-time password recovery |
//! | PWA | [`plugins::pwa`] | Manifest, service worker, offline page |
//! | Export | [`plugins::export`] | XLSX data export UI |
//! | No signup | [`plugins::no_signup`] | Disables public signup routes |
//!
//! Proc-macro derives expand to `::lariv_rs::…` paths; this crate aliases itself as `lariv_rs` for in-tree use.
extern crate self as lariv_rs;

pub mod app;
pub mod apps;
pub mod capability;
pub mod command;
pub mod components;
pub mod config;
pub mod db;
pub mod datetime;
pub mod duration;
pub mod docs;
pub mod export;
pub mod genai;
pub mod grapesjs;
pub mod hooks;
pub mod html_form;
pub mod http;
pub mod layers;
#[cfg(feature = "cap-llm")]
pub mod llm_tools;
#[cfg(not(feature = "cap-llm"))]
#[path = "llm_tools_stub.rs"]
pub mod llm_tools;
#[cfg(feature = "cap-llm")]
pub mod rune_env;
#[cfg(not(feature = "cap-llm"))]
#[path = "rune_env_stub.rs"]
pub mod rune_env;
pub mod migration;
pub mod picker;
pub mod plugin_install;
pub mod plugin_routes;
pub mod plugins;
pub mod tag;
pub mod template;
pub mod traits;
pub mod views;
pub mod web;

/// Generate route tags, proof type, and [`RouteRegistrar`](http::RouteRegistrar) hook.
///
/// See [`plugin_routes`] for the full DSL reference.
pub use lariv_rs_macros::define_plugin_routes;

/// Re-exported for [`define_plugin_install!`](plugin_install::define_plugin_install) `cap_attach` /
/// `cap_hook` unique type-parameter names (used via `$crate::paste` from the macro).
pub use paste;
