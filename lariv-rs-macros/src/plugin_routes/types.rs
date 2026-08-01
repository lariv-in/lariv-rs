//! Resolve path param names to Rust field types.

use proc_macro2::Span;
use syn::{Ident, Type, spanned::Spanned};

use super::path::ParsedPath;

#[derive(Clone)]
pub struct ResolvedParam {
    pub name: Ident,
    pub ty: Type,
    pub splat: bool,
}

/// Convention: `{*name}` → `Vec<String>`, `{id}` / `{*_id}` → `i64`, else `String`.
pub fn default_param_type(name: &str, splat: bool) -> Type {
    if splat {
        return syn::parse_quote!(::std::vec::Vec<::std::string::String>);
    }
    if name == "id" || name.ends_with("_id") {
        return syn::parse_quote!(i64);
    }
    syn::parse_quote!(::std::string::String)
}

pub fn resolve_params(
    parsed: &ParsedPath,
    overrides: &[(Ident, Type)],
    route_span: Span,
) -> syn::Result<Vec<ResolvedParam>> {
    let mut seen = std::collections::HashSet::new();
    let mut params = Vec::new();

    for seg in &parsed.segments {
        let super::path::PathSegment::Param { name, splat } = seg else {
            continue;
        };
        if !seen.insert(name.clone()) {
            return Err(syn::Error::new(
                route_span,
                format!("duplicate path param `{name}`"),
            ));
        }
        let ident = Ident::new(name, route_span);
        let ty = overrides
            .iter()
            .find(|(n, _)| n == &ident)
            .map(|(_, ty)| ty.clone())
            .unwrap_or_else(|| default_param_type(name, *splat));
        params.push(ResolvedParam {
            name: ident,
            ty,
            splat: *splat,
        });
    }

    for (name, ty) in overrides {
        if !params.iter().any(|p| p.name == *name) {
            return Err(syn::Error::new(
                ty.span(),
                format!("param `{name}` is not in route path"),
            ));
        }
    }

    Ok(params)
}
