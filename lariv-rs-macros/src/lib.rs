//! Procedural macros for `lariv-rs`.

mod html_form;
mod plugin_routes;

use proc_macro::TokenStream;

/// Attribute macro: owns serde field wiring and emits [`HtmlForm`](lariv_rs::html_form::HtmlForm).
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

/// Generate route tags, proof type, and [`RouteRegistrar`](lariv_rs::http::RouteRegistrar) hook.
#[proc_macro]
pub fn define_plugin_routes(input: TokenStream) -> TokenStream {
    plugin_routes::define_plugin_routes(input)
}
