//! Parse `define_plugin_routes! { ... }` DSL.

use proc_macro2::Span;
use syn::{
    Ident, Path, Token, Type, bracketed, parse::Parse, parse::ParseStream, spanned::Spanned,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
}

#[derive(Clone)]
pub enum ResponseKind {
    Pane,
    Modal,
    Fragment(Type),
    File,
    Redirect,
    Generation,
    Raw,
}

#[derive(Clone)]
pub struct RouteSpec {
    pub method: HttpMethod,
    pub tag: Ident,
    pub path: String,
    pub path_span: Span,
    pub handler: Path,
    pub response: ResponseKind,
    pub param_overrides: Vec<(Ident, Type)>,
}

#[derive(Clone)]
pub struct PluginRoutesInput {
    pub routes: Vec<RouteSpec>,
}

impl Parse for PluginRoutesInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut plugin: Option<Type> = None;
        let mut routes = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![:]>()?;

            if key == "plugin" {
                plugin = Some(input.parse()?);
            } else if key == "proof" {
                let _: Ident = input.parse()?;
            } else if key == "slots" {
                let _: Ident = input.parse()?;
            } else if key == "pages" {
                let content;
                bracketed!(content in input);
                while !content.is_empty() {
                    if content.peek(Ident) {
                        let word: Ident = content.parse()?;
                        if word == "pane" || word == "page" {
                            let _: Ident = content.parse()?;
                            content.parse::<Token![,]>()?;
                            let _: Ident = content.parse()?;
                            content.parse::<Token![=>]>()?;
                            let _: Type = content.parse()?;
                            content.parse::<Token![,]>()?;
                            let _: Type = content.parse()?;
                        }
                    }
                    if content.peek(Token![;]) {
                        content.parse::<Token![;]>()?;
                    }
                }
            } else if key == "routes" {
                let content;
                bracketed!(content in input);
                routes = parse_routes(&content)?;
            } else {
                return Err(syn::Error::new(key.span(), "unknown key"));
            }
            if !input.is_empty() {
                input.parse::<Token![;]>()?;
            }
        }

        let Some(_plugin) = plugin else {
            return Err(input.error("missing `plugin:`"));
        };
        if routes.is_empty() {
            return Err(input.error("missing `routes:`"));
        }

        Ok(Self { routes })
    }
}

fn parse_routes(input: ParseStream<'_>) -> syn::Result<Vec<RouteSpec>> {
    let mut routes = Vec::new();
    while !input.is_empty() {
        routes.push(parse_route_line(input)?);
    }
    Ok(routes)
}

fn parse_route_line(input: ParseStream<'_>) -> syn::Result<RouteSpec> {
    let method_ident: Ident = input.parse()?;
    let method = if method_ident == "get" {
        HttpMethod::Get
    } else if method_ident == "post" {
        HttpMethod::Post
    } else {
        return Err(syn::Error::new(
            method_ident.span(),
            "expected `get` or `post`",
        ));
    };

    let tag: Ident = input.parse()?;
    input.parse::<Token![,]>()?;
    let lit: syn::LitStr = input.parse()?;
    let path = lit.value();
    let path_span = lit.span();
    input.parse::<Token![,]>()?;

    let bare = if input.peek(Ident) {
        let fork = input.fork();
        fork.parse::<Ident>().is_ok_and(|w| w == "bare")
    } else {
        false
    };
    if bare {
        input.parse::<Ident>()?;
    }

    let handler: Path = input.parse()?;

    let mut response = None;
    let mut param_overrides = Vec::new();

    while input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
        if input.peek(Ident) && {
            let fork = input.fork();
            let Ok(word) = fork.parse::<Ident>() else {
                return Err(input.error("expected response kind or `param`"));
            };
            word == "param"
        } {
            input.parse::<Ident>()?; // param
            let name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let ty: Type = input.parse()?;
            if param_overrides.iter().any(|(n, _)| n == &name) {
                return Err(syn::Error::new(name.span(), "duplicate `param`"));
            }
            param_overrides.push((name, ty));
            continue;
        }

        if response.is_some() {
            return Err(input.error("duplicate response kind"));
        }
        response = Some(parse_response(input, bare)?);
    }

    input.parse::<Token![;]>()?;

    let response = match (bare, response) {
        (true, None) => {
            return Err(syn::Error::new(
                handler.span(),
                "bare route must specify a response kind: file, modal, redirect, generation, raw, or fragment(SwapKey)",
            ));
        }
        (false, None) => ResponseKind::Pane,
        (_, Some(r)) => r,
    };

    Ok(RouteSpec {
        method,
        tag,
        path,
        path_span,
        handler,
        response,
        param_overrides,
    })
}

fn parse_response(input: ParseStream<'_>, _bare: bool) -> syn::Result<ResponseKind> {
    let ident: Ident = input.parse()?;
    if ident == "modal" {
        return Ok(ResponseKind::Modal);
    }
    if ident == "file" {
        return Ok(ResponseKind::File);
    }
    if ident == "redirect" {
        return Ok(ResponseKind::Redirect);
    }
    if ident == "generation" {
        return Ok(ResponseKind::Generation);
    }
    if ident == "raw" {
        return Ok(ResponseKind::Raw);
    }
    if ident == "fragment" {
        let frag;
        syn::parenthesized!(frag in input);
        let ty: Type = frag.parse()?;
        return Ok(ResponseKind::Fragment(ty));
    }
    Err(syn::Error::new(
        ident.span(),
        "expected modal, file, redirect, generation, raw, or fragment(Type)",
    ))
}
