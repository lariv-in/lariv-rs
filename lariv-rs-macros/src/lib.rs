//! Procedural macros for `lariv-rs`.
//!
//! # Macros
//!
//! - [`html_form`] — attribute macro: serde field wiring + `HtmlForm` trait impl
//! - [`define_plugin_routes`] — proc macro: route tags, URL builders, response traits, `RouteRegistrar` hook
//! - [`define_register_items`] — proc macro: item tags and a registrar hook
//! - [`define_replace_templates`] — proc macro: replace and prepend template registrations
//! - [`define_passthrough_cap`] — proc macro: hook-free state capability
//! - [`define_register_export`] — proc macro: `ExportRegistrar` hook for one table
//! - [`define_plugin_install`] — proc macro: plugin `install` function
//! - [`apply_registrar_hook`] — proc macro: `ApplyHooks` + `FoldRegistrarHooks` for a registrar
//! - [`main`] — attribute macro: large-stack async `main` for deep HList install/mount

mod capability;
mod html_form;
mod main_attr;
mod plugin_install;
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
/// - `search_key = "..."` — foreign-key typeahead query parameter (default `Name`)
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
///     // `csrf: CsrfToken` is injected by the macro
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
///     prefix: "/dashboard";       // optional — prepended to route paths unless `root`
///     routes: [
///         get RouteTag, "/path", handler::fn;
///         get PublicRouteTag, "/public", root bare handler::fn, raw;
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
/// `{get|post} Tag, "path-literal", [root] [bare] handler_path [, response] [, param name: Type]* ;`
///
/// - **`root`** — keep the path at site root (skip `prefix`, for public/auth routes).
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
///         get UsersListRouteTag, "/users", handlers::users::list, fragment(USER_TABLE);
///     ]
/// }
/// ```
#[proc_macro]
pub fn define_plugin_routes(input: TokenStream) -> TokenStream {
    plugin_routes::define_plugin_routes(input)
}

/// `ApplyHooks` and `FoldRegistrarHooks` for one registrar trait.
///
/// Used by the template, slot, migration, and command capabilities. The `proof`
/// type is the token carried in each hook's proof tuple; `tag` is the capability tag.
#[proc_macro]
pub fn apply_registrar_hook(input: TokenStream) -> TokenStream {
    expand(capability::apply_registrar_hook(input.into()))
}

/// Declare item tags and a `Register*` impl for a marker-style capability.
///
/// Two item shapes are supported (add-order; last entry is the HList head):
///
/// - `Tag => Value` — `add::<Tag, Value>()` → `Tagged<Tag, Wrapper<Value>>` (templates)
/// - `Tag, Kind => Value` — `add::<Tag, Kind, Value>()` → `Tagged<Tag, Wrapper<Kind, Value>>` (slots)
/// - empty `items: []` — identity `Register*` (hook queued, nothing added)
///
/// Index names before each tag are call-site documentation; frunk indices are inferred.
#[proc_macro]
pub fn define_register_items(input: TokenStream) -> TokenStream {
    expand(capability::define_register_items(input.into()))
}

/// Register a template hook that replaces tagged pages and optionally prepends new ones.
#[proc_macro]
pub fn define_replace_templates(input: TokenStream) -> TokenStream {
    expand(capability::define_replace_templates(input.into()))
}

/// `CapStore<Tag, HNil, Value>` that mounts as `Tagged::new(items)`.
#[proc_macro]
pub fn define_passthrough_cap(input: TokenStream) -> TokenStream {
    expand(capability::define_passthrough_cap(input.into()))
}

/// Register one exportable table as an `ExportRegistrar` hook.
///
/// ```ignore
/// define_register_export! {
///     plugin: UsersTag;
///     table: "users";
///     model: "User";
///     columns: ["id", "email"];
///     deps: ["clients"];
/// }
/// ```
///
/// `deps` is optional. `plugin` is required for call-site consistency and is not
/// emitted; the generated `ExportHook` does not name the plugin tag.
#[proc_macro]
pub fn define_register_export(input: TokenStream) -> TokenStream {
    expand(capability::define_register_export(input.into()))
}

/// Generate a plugin `install` function and the `After*` type aliases it chains.
///
/// Re-exported from `lariv-core` as [`define_plugin_install`](self::define_plugin_install).
/// Call sites keep `lariv_core::plugin_install::define_plugin_install!`.
///
/// ```ignore
/// define_plugin_install! {
///     plugin: UsersTag;
///     /// Register users deferred hooks and config section.
///     steps: [
///         apps(apps::Hook),
///         migrations(migrations::Hook),
///         http(routes::Hook),
///         state(StateHook),
///     ]
/// }
/// ```
#[proc_macro]
pub fn define_plugin_install(input: TokenStream) -> TokenStream {
    expand(plugin_install::define_plugin_install(input.into()))
}

fn expand(result: syn::Result<proc_macro2::TokenStream>) -> TokenStream {
    match result {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Run `async fn main` on a thread with a raised stack size.
///
/// Re-exported from `lariv-rs` as [`lariv_rs::main`]. See that crate for attributes
/// and examples.
#[proc_macro_attribute]
pub fn main(attr: TokenStream, item: TokenStream) -> TokenStream {
    main_attr::main_attr(attr, item)
}
