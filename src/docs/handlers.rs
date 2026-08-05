//! Axum HTTP handlers — request entry points for routes.
//!
//! # Handler basics
//!
//! Each route points to an `async fn` in `handlers.rs` (or `handlers/` submodule).
//! Handlers extract dependencies from Axum and return HTML, redirects, or files.
//!
//! ```ignore
//! use lariv_rs::{
//!     components::{SharedChromeFolder, SlotCtx},
//!     http::Cap,
//!     web::{Htmx, html_built_page_or_app_layout},
//! };
//! use super::templates::ListPage;
//!
//! pub async fn list(
//!     Cap(chrome): Cap<SharedChromeFolder>,
//!     htmx: Htmx,
//! ) -> maud::Markup {
//!     let page = ListPage { items: vec![] };
//!     html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::default())
//! }
//! ```
//!
//! # Extracting plugin state
//!
//! Mounted plugin state is available via [`Cap<T>`](crate::http::Cap):
//!
//! ```ignore
//! use lariv_rs::http::Cap;
//! use super::MyState;
//!
//! pub async fn list(Cap(state): Cap<MyState>) -> impl IntoResponse {
//!     let rows = my_entity::Entity::find().all(&state.db).await?;
//!     // …
//! }
//! ```
//!
//! # Authentication
//!
//! The users plugin provides middleware extractors:
//!
//! | Extractor | Behavior |
//! |-----------|----------|
//! | `RequireAuth` | 401/redirect if not logged in; provides `AuthCtx` |
//! | `OptionalAuth` | `None` for guests; used for public pages with optional user context |
//! | `RequireRole("admin")` | Role check after authentication |
//!
//! ```ignore
//! use lariv_rs::plugins::users::middleware::{RequireAuth, OptionalAuth};
//!
//! pub async fn admin_only(RequireAuth(ctx): RequireAuth) -> impl IntoResponse {
//!     // ctx.user, ctx.role available
//! }
//! ```
//!
//! # HTMX responses
//!
//! [`Htmx`](crate::web::Htmx) detects partial requests. Use
//! [`html_built_page_or_app_layout`](crate::web::html_built_page_or_app_layout) for pages that
//! support both full loads and fragment swaps.
//!
//! For table filter/sort pagination, routes declare `fragment(SwapKey)` in
//! [`define_plugin_routes!`](crate::define_plugin_routes) and return markup targeting that region.
//!
//! # Query string pagination
//!
//! Use [`QueryPage`](crate::web::QueryPage) for `page` fields and [`QueryI64`](crate::web::QueryI64)
//! for optional ID filter fields on axum [`Query`] structs (especially inside
//! `#[serde(flatten)]` list filters and FK picker routes). Raw `Option<u32>` / `Option<i64>`
//! break on empty query values — see [`crate::web::query`].
//!
//! # Create/edit POST forms
//!
//! Forms that use [`form_hx_post_main`](crate::components::form_hx_post_main) must target
//! [`AppPanePost`](crate::http::AppPanePost) routes. Handlers re-render the form pane with
//! `form_error` on validation/persistence failure and call [`Htmx::redirect`](crate::web::Htmx::redirect)
//! on success. Redirect-only handlers ([`BoostPost`](crate::http::BoostPost) — delete, logout)
//! use [`form_hx_post_redirect`](crate::components::form_hx_post_redirect) instead.
//!
//! # View layers vs handlers
//!
//! For CRUD pages that load records, validate forms, and redirect, prefer composing
//! [`View`](crate::layers::View) layer stacks instead of hand-rolling handler logic.
//! See [`super::layers`].
//!
//! Handlers remain the HTTP entry point — layers run inside them via
//! [`run_layers`](crate::layers::run_layers) or the default route wrapper.
