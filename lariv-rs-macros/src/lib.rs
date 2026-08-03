//! Procedural macros for `lariv-rs`.
//!
//! # Macros
//!
//! - [`html_form`] — attribute macro: serde field wiring + `HtmlForm` trait impl
//! - [`define_plugin_routes`] — proc macro: route tags, URL builders, response traits, `RouteRegistrar` hook

mod html_form;
mod plugin_routes;

use proc_macro::TokenStream;

/// Attribute macro: owns serde field wiring and emits a `HtmlForm` trait implementation.
///
/// Applied to a struct or tagged enum. Field attributes use `#[form(...)]`:
///
/// - `label = "..."` — display label
/// - `widget = Text | Email | Password | ...` — HTML widget type
/// - `required` — non-empty validation
/// - `name = "..."` — override HTML `name` attribute
/// - `row = "..."` — group fields in the same form row
///
/// Macro args (on the attribute itself): `default`, `no_debug`, `tag = "..."`.
///
/// # Examples
///
/// ```ignore
/// #[html_form]
/// pub struct MyForm {
///     #[form(label = "Name", widget = Text, required)]
///     pub name: String,
/// }
/// ```
#[proc_macro_attribute]
pub fn html_form(attr: TokenStream, item: TokenStream) -> TokenStream {
    html_form::html_form_attr(attr, item)
}

/// Generate route tags, proof type, and a `RouteRegistrar` hook.
///
/// # DSL
///
/// ```text
/// define_plugin_routes! {
///     plugin: PluginTag;           // required — plugin identity (for hook tagging)
///     proof: ProofName;            // optional — reserved for future compile-time proofs
///     slots: SlotCtxTy;            // optional — reserved for page/slot wiring
///     pages: [ ... ];              // optional — reserved pane/page declarations
///     routes: [
///         get RouteTag, "/path", handler::fn;
///         post RouteTag, "/path/{id}", handler::fn, modal;
///         get RouteTag, "/path/{*tail}", bare handler::fn, redirect;
///         post RouteTag, "/path", bare handler::fn, fragment(SwapKeyTy);
///         post RouteTag, "/path", bare handler::fn, file;
///         post RouteTag, "/path", bare handler::fn, generation;
///         get RouteTag, "/path", bare handler::fn, raw;
///         get RouteTag, "/users/u/{id}", handler::fn, param id: i64;
///     ]
/// }
/// ```
///
/// ## Route line syntax
///
/// `{get|post} Tag, "path-literal", [bare] handler_path [, response] [, param name: Type]* ;`
///
/// - **`bare`** — handler is registered without the default view stack wrapper (required
///   when specifying a non-default response kind).
/// - **Response kinds** (default: `pane` for GET, `pane` for POST):
///   - `modal` — HTMX modal overlay
///   - `fragment(SwapKey)` — partial HTML swap keyed by `SwapKey`
///   - `file` — file download response
///   - `redirect` — redirect response (GET: pane redirect; POST: boost redirect)
///   - `generation` — streaming/generation POST
///   - `raw` — unwrapped handler response
/// - **`param name: Type`** — override inferred path param type (default: `{id}` → `i64`,
///   `{*name}` → `Vec<String>`, else `String`).
///
/// ## Generated items (per route)
///
/// - `RouteTag` struct — URL builder with `PATH`, `new(...)`, `path()`, `url()`, `with_query()`
/// - `RouteTag: RouteTag, RouteUrl` — path metadata
/// - Response marker traits — `AppPaneGet/Post`, `ModalGet`, `FragmentGet/Post`, etc.
/// - `Hook: RouteRegistrar<HttpCapability<R>>` — prepends routes onto the HTTP capability
///
/// # Examples
///
/// ```ignore
/// define_plugin_routes! {
///     plugin: UsersTag;
///     routes: [
///         get UsersLoginGetRouteTag, "/users/login", handlers::auth::login_get;
///         post UsersLoginPostRouteTag, "/users/login", handlers::auth::login_post;
///         get UsersLogoutGetRouteTag, "/users/logout", bare handlers::auth::logout, redirect;
///         get UsersListRouteTag, "/users", handlers::users::list, fragment(UserTableKey);
///     ]
/// }
/// ```
#[proc_macro]
pub fn define_plugin_routes(input: TokenStream) -> TokenStream {
    plugin_routes::define_plugin_routes(input)
}
