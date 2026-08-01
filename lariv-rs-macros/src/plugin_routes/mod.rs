mod emit;
mod parse;
mod path;
mod types;

use proc_macro::TokenStream;
use syn::parse_macro_input;

pub fn define_plugin_routes(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as parse::PluginRoutesInput);
    match emit::expand(&parsed) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
