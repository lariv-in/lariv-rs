//! HTTP routing with `define_plugin_routes!`.
//!
//! # Registering routes (`routes.rs`)
//!
//! Lariv routes map URL path patterns to async Axum handlers. Declare them in the plugin's
//! `routes.rs` using [`define_plugin_routes!`](crate::define_plugin_routes):
//!
//! ```ignore
//! use lariv_rs::define_plugin_routes;
//! use super::handlers;
//!
//! define_plugin_routes! {
//!     plugin: MyPluginTag;
//!     routes: [
//!         // Standard app-pane GET (full page or HTMX partial)
//!         get ListRouteTag, "/items", handlers::list;
//!
//!         // POST with modal response
//!         post CreatePostRouteTag, "/items/create", handlers::create_post, modal;
//!
//!         // GET with HTMX fragment swap into a table region
//!         get ListRouteTag, "/items", handlers::list, fragment(ItemTableKey);
//!
//!         // Bare handler — you choose the response type
//!         get LogoutRouteTag, "/logout", bare handlers::logout, redirect;
//!         post DownloadRouteTag, "/export", bare handlers::download, file;
//!     ]
//! }
//! ```
//!
//! # Route line syntax
//!
//! ```text
//! {get|post} RouteTag, "path/{param}", [bare] handler::fn [, response] [, param name: Type] ;
//! ```
//!
//! - **`bare`** — skip the default view wrapper; required when specifying `file`, `redirect`,
//!   `raw`, or custom `fragment` responses.
//! - **Response kinds** — `modal`, `fragment(SwapKey)`, `file`, `redirect`, `generation`, `raw`.
//!   Default for GET/POST app routes is app-pane rendering.
//! - **Path params** — `{id}` → `i64`, `{slug}` → `String`, `{*tail}` → `Vec<String>`.
//!   Override with `param id: i64`.
//!
//! # Generated items
//!
//! For each route the macro creates:
//!
//! - `RouteTag` struct — typed URL builder with `PATH`, `new(…)`, `.path()`, `.url()`
//! - Response marker traits — `AppPaneGet`, `FragmentGet`, `ModalPost`, etc.
//! - `Hook: RouteRegistrar<…>` — prepends routes during plugin install
//!
//! At mount time, later-installed plugins win on duplicate `(path, method)` —
//! so a public-site plugin installed after dashboard can own `/`.
//!
//! Use route tags in templates instead of hard-coded paths:
//!
//! ```ignore
//! use super::routes::ItemDetailRouteTag;
//!
//! html! {
//!     a href=(ItemDetailRouteTag::new(item.id).url()) { (item.name) }
//! }
//! ```
//!
//! # Typed query strings
//!
//! [`RouteQueryBuilder`](crate::http::route_tag::RouteQueryBuilder) helpers attach filters
//! to list URLs (sort, page, search):
//!
//! ```ignore
//! ItemListRouteTag::new()
//!     .with_query(&[("sort", "name"), ("page", "2")])
//!     .url()
//! ```
//!
//! # Handler signature
//!
//! Non-bare handlers receive template/slot context via the default wrapper. Bare handlers
//! are plain Axum functions — see [`super::handlers`].
