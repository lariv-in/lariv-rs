//! Procedural macros for `lariv-rs`.
//!
//! # Macros
//!
//! - [`html_form`] — attribute macro: serde field wiring + `HtmlForm` trait impl
//! - [`define_plugin_routes`] — proc macro: route tags, URL builders, response traits, `RouteRegistrar` hook
//! - [`main`] — attribute macro: large-stack async `main` for deep HList install/mount

mod html_form;
mod main_attr;
mod plugin_routes;

use proc_macro::TokenStream;
use proc_macro2::{Group, Ident, TokenTree};
use quote::quote;
use syn::Item;

/// Emit each item twice so plugin crates resolve `::lariv_core::` and downstream
/// crates keep resolving `::lariv_rs::`.
fn dual_crate(ts: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let core_ts = rewrite_users_plugin_path(rename_lariv_rs(ts.clone()));
    let facade = match syn::parse2::<syn::File>(ts) {
        Ok(file) => file,
        Err(err) => {
            let msg = err.to_string();
            return quote! { compile_error!(#msg); };
        }
    };
    let core = match syn::parse2::<syn::File>(core_ts) {
        Ok(file) => file,
        Err(err) => {
            let msg = err.to_string();
            return quote! { compile_error!(#msg); };
        }
    };
    let mut out = proc_macro2::TokenStream::new();
    for (facade_item, core_item) in facade.items.into_iter().zip(core.items) {
        out.extend(cfg_item(false, facade_item));
        out.extend(cfg_item(true, core_item));
    }
    out
}

fn cfg_item(plugin: bool, item: Item) -> proc_macro2::TokenStream {
    if plugin {
        quote! {
            #[allow(unexpected_cfgs)]
            #[cfg(lariv_plugin_crate)]
            #item
        }
    } else {
        quote! {
            #[allow(unexpected_cfgs)]
            #[cfg(not(lariv_plugin_crate))]
            #item
        }
    }
}

fn rename_lariv_rs(ts: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    ts.into_iter()
        .map(|tree| match tree {
            TokenTree::Ident(id) if id == "lariv_rs" => {
                TokenTree::Ident(Ident::new("lariv_core", id.span()))
            }
            TokenTree::Literal(lit) => {
                let raw = lit.to_string();
                if raw.contains("lariv_rs") {
                    let replaced = raw.replace("lariv_rs", "lariv_core");
                    let parsed: proc_macro2::TokenStream = replaced
                        .parse()
                        .unwrap_or_else(|_| proc_macro2::TokenStream::from(TokenTree::Literal(lit.clone())));
                    return parsed
                        .into_iter()
                        .next()
                        .unwrap_or(TokenTree::Literal(lit));
                }
                TokenTree::Literal(lit)
            }
            TokenTree::Group(group) => {
                let mut renamed = Group::new(group.delimiter(), rename_lariv_rs(group.stream()));
                renamed.set_span(group.span());
                TokenTree::Group(renamed)
            }
            other => other,
        })
        .collect()
}

/// `::lariv_core::plugins::users` (from `::lariv_rs::plugins::users`) is the users plugin crate.
fn rewrite_users_plugin_path(ts: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let tokens: Vec<TokenTree> = ts.into_iter().collect();
    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if let TokenTree::Group(group) = &tokens[i] {
            let mut renamed = Group::new(group.delimiter(), rewrite_users_plugin_path(group.stream()));
            renamed.set_span(group.span());
            out.push(TokenTree::Group(renamed));
            i += 1;
            continue;
        }
        if matches_users_path(&tokens[i..]) {
            let span = match &tokens[i] {
                TokenTree::Ident(id) => id.span(),
                _ => proc_macro2::Span::call_site(),
            };
            out.push(TokenTree::Ident(Ident::new("lariv_plugin_users", span)));
            i += 7;
            continue;
        }
        out.push(tokens[i].clone());
        i += 1;
    }
    out.into_iter().collect()
}

fn matches_users_path(tokens: &[TokenTree]) -> bool {
    if tokens.len() < 7 {
        return false;
    }
    let idents = ["lariv_core", "plugins", "users"];
    let steps = [
        Some(idents[0]),
        None,
        None,
        Some(idents[1]),
        None,
        None,
        Some(idents[2]),
    ];
    for (token, expect) in tokens.iter().take(7).zip(steps) {
        match expect {
            Some(name) => {
                let TokenTree::Ident(id) = token else {
                    return false;
                };
                if id != name {
                    return false;
                }
            }
            None => {
                let TokenTree::Punct(punct) = token else {
                    return false;
                };
                if punct.as_char() != ':' {
                    return false;
                }
            }
        }
    }
    true
}

fn emit(ts: proc_macro2::TokenStream) -> TokenStream {
    let rendered = ts.to_string();
    if rendered.contains("compile_error") {
        return ts.into();
    }
    dual_crate(ts).into()
}

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
///     // `csrf: CsrfToken` is injected by the macro
/// }
/// ```
#[proc_macro_attribute]
pub fn html_form(attr: TokenStream, item: TokenStream) -> TokenStream {
    emit(html_form::html_form_attr(attr, item).into())
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
///         get UsersListRouteTag, "/users", handlers::users::list, fragment(UserTableKey);
///     ]
/// }
/// ```
#[proc_macro]
pub fn define_plugin_routes(input: TokenStream) -> TokenStream {
    emit(plugin_routes::define_plugin_routes(input).into())
}

/// Run `async fn main` on a thread with a raised stack size.
///
/// Re-exported from `lariv-rs` as [`lariv_rs::main`]. See that crate for attributes
/// and examples.
#[proc_macro_attribute]
pub fn main(attr: TokenStream, item: TokenStream) -> TokenStream {
    emit(main_attr::main_attr(attr, item).into())
}
