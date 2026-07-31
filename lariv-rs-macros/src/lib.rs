//! Procedural macros for `lariv-rs`.

mod html_form;

use proc_macro::TokenStream;

/// Attribute macro: owns serde field wiring and emits [`lariv_rs::html_form::HtmlForm`].
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
