//! Generate route tags, traits, and RouteRegistrar hook.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Ident;

use super::parse::{HttpMethod, PluginRoutesInput, ResponseKind, RouteSpec};
use super::path::{PathSegment, parse_path};
use super::types::{ResolvedParam, resolve_params};

pub fn expand(input: &PluginRoutesInput) -> syn::Result<TokenStream2> {
    let route_items: Vec<_> = input
        .routes
        .iter()
        .map(emit_route_tag)
        .collect::<syn::Result<_>>()?;

    let hook = emit_hook(input);

    Ok(quote! {
        #(#route_items)*
        #hook
    })
}

fn emit_route_tag(route: &RouteSpec) -> syn::Result<TokenStream2> {
    let tag = &route.tag;
    let path_lit = &route.path;
    let parsed = parse_path(&route.path);
    let params = resolve_params(&parsed, &route.param_overrides, route.path_span)?;
    let trait_impls = emit_response_traits(route);
    let param_name_strs: Vec<_> = params.iter().map(|p| p.name.to_string()).collect();

    let struct_def;
    let inherent_impl;
    let route_url_impl;

    if params.is_empty() {
        struct_def = quote! {
            #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::default::Default)]
            pub struct #tag;
        };
        inherent_impl = quote! {
            impl #tag {
                pub const PATH: &'static str = #path_lit;
                pub fn path(self) -> ::std::string::String {
                    Self::PATH.to_owned()
                }
                pub fn url(self) -> ::std::string::String {
                    ::lariv_rs::http::route_tag::trailing_slash(&self.path())
                }
                pub fn with_query(self) -> ::lariv_rs::http::RouteQueryBuilder<Self> {
                    ::lariv_rs::http::RouteQueryBuilder::new(self)
                }
            }
        };
        route_url_impl = quote! {
            impl ::lariv_rs::http::RouteUrl for #tag {
                fn path(self) -> ::std::string::String {
                    <Self>::path(self)
                }
                fn url(self) -> ::std::string::String {
                    <Self>::url(self)
                }
            }
        };
    } else {
        let fields = params.iter().map(|p| {
            let name = &p.name;
            let ty = &p.ty;
            quote! { pub #name: #ty }
        });
        let new_params = params.iter().map(|p| {
            let name = &p.name;
            let ty = &p.ty;
            quote! { #name: #ty }
        });
        let field_names: Vec<_> = params.iter().map(|p| &p.name).collect();
        let path_body = emit_path_body(&parsed, &params);
        let splat_from = emit_splat_from(tag, &params);

        struct_def = quote! {
            #[derive(::core::clone::Clone, ::core::fmt::Debug)]
            pub struct #tag {
                #(#fields,)*
            }
        };
        inherent_impl = quote! {
            impl #tag {
                pub const PATH: &'static str = #path_lit;
                pub fn new(#(#new_params,)*) -> Self {
                    Self { #(#field_names: #field_names,)* }
                }
                pub fn path(self) -> ::std::string::String {
                    #path_body
                }
                pub fn url(self) -> ::std::string::String {
                    ::lariv_rs::http::route_tag::trailing_slash(&self.path())
                }
                pub fn with_query(self) -> ::lariv_rs::http::RouteQueryBuilder<Self> {
                    ::lariv_rs::http::RouteQueryBuilder::new(self)
                }
            }
            #splat_from
        };
        route_url_impl = quote! {
            impl ::lariv_rs::http::RouteUrl for #tag {
                fn path(self) -> ::std::string::String {
                    <Self>::path(self)
                }
                fn url(self) -> ::std::string::String {
                    <Self>::url(self)
                }
            }
        };
    }

    Ok(quote! {
        #struct_def
        #inherent_impl
        impl ::lariv_rs::http::RouteTag for #tag {
            const PATH: &'static str = #path_lit;
            const PARAMS: &'static [&'static str] = &[#(#param_name_strs),*];
        }
        #route_url_impl
        #trait_impls
    })
}

fn emit_path_body(parsed: &super::path::ParsedPath, params: &[ResolvedParam]) -> TokenStream2 {
    if params.len() == 1 && params[0].splat {
        let name = &params[0].name;
        let prefix: String = parsed
            .segments
            .iter()
            .filter_map(|s| match s {
                PathSegment::Static(st) => Some(st.as_str()),
                PathSegment::Param { .. } => None,
            })
            .collect();
        if prefix.is_empty() || prefix == "/" {
            return quote! {
                {
                    let tail = self.#name.join("/");
                    if tail.is_empty() {
                        "/".to_owned()
                    } else {
                        format!("/{tail}")
                    }
                }
            };
        }
        let prefix = prefix.trim_end_matches('/').to_string();
        return quote! {
            {
                let tail = self.#name.join("/");
                if tail.is_empty() {
                    #prefix.to_owned()
                } else {
                    ::std::format!(::core::concat!(#prefix, "/{}"), tail)
                }
            }
        };
    }

    let mut format_lit = String::new();
    let mut format_args = Vec::new();

    for seg in &parsed.segments {
        match seg {
            PathSegment::Static(st) => format_lit.push_str(st),
            PathSegment::Param { name, splat: false } => {
                format_lit.push_str("{}");
                let ident = Ident::new(name, proc_macro2::Span::call_site());
                format_args.push(quote! { self.#ident });
            }
            PathSegment::Param { name, splat: true } => {
                let ident = Ident::new(name, proc_macro2::Span::call_site());
                format_lit.push_str("{}");
                format_args.push(quote! { self.#ident.join("/") });
            }
        }
    }

    quote! {
        ::std::format!(#format_lit #(, #format_args)*)
    }
}

fn emit_splat_from(tag: &Ident, params: &[ResolvedParam]) -> TokenStream2 {
    let Some(splat) = params.iter().find(|p| p.splat) else {
        return quote! {};
    };
    if params.len() != 1 {
        return quote! {};
    }
    let name = &splat.name;
    quote! {
        impl ::core::convert::From<::std::string::String> for #tag {
            fn from(s: ::std::string::String) -> Self {
                Self {
                    #name: if s.is_empty() {
                        ::std::vec::Vec::new()
                    } else {
                        s.split('/').map(str::to_owned).collect()
                    },
                }
            }
        }
    }
}

fn emit_response_traits(route: &RouteSpec) -> TokenStream2 {
    let tag = &route.tag;
    match (&route.response, route.method) {
        (ResponseKind::Modal, _) => quote! {
            impl ::lariv_rs::http::ModalGet for #tag {}
        },
        (ResponseKind::Pane, HttpMethod::Get) => quote! {
            impl ::lariv_rs::http::AppPaneGet for #tag {}
        },
        (ResponseKind::Pane, HttpMethod::Post) => quote! {
            impl ::lariv_rs::http::AppPanePost for #tag {}
        },
        (ResponseKind::Fragment(ty), HttpMethod::Get) => quote! {
            impl ::lariv_rs::http::AppPaneGet for #tag {}
            impl ::lariv_rs::http::FragmentGet<#ty> for #tag {}
        },
        (ResponseKind::FkSelect(table, modal), HttpMethod::Get) => quote! {
            impl ::lariv_rs::http::AppPaneGet for #tag {}
            impl ::lariv_rs::http::FragmentGet<#table> for #tag {}
            impl ::lariv_rs::http::FkSelectGet<#table, #modal> for #tag {}
        },
        (ResponseKind::FkSelect(_, _), HttpMethod::Post) => quote! {},
        (ResponseKind::Fragment(ty), HttpMethod::Post) => quote! {
            impl ::lariv_rs::http::AppPanePost for #tag {}
            impl ::lariv_rs::http::FragmentPost<#ty> for #tag {}
        },
        (ResponseKind::File, HttpMethod::Get) => quote! {
            impl ::lariv_rs::http::FileDownloadGet for #tag {}
        },
        (ResponseKind::File, HttpMethod::Post) => quote! {
            impl ::lariv_rs::http::FileDownloadPost for #tag {}
        },
        (ResponseKind::Redirect, HttpMethod::Get) => quote! {
            impl ::lariv_rs::http::AppPaneGet for #tag {}
        },
        (ResponseKind::Redirect, HttpMethod::Post) => quote! {
            impl ::lariv_rs::http::BoostPost for #tag {}
        },
        (ResponseKind::Generation, _) => quote! {
            impl ::lariv_rs::http::GenerationPost for #tag {}
        },
        (ResponseKind::Raw, _) => quote! {},
    }
}

fn emit_hook(input: &PluginRoutesInput) -> TokenStream2 {
    let rev_tags: Vec<_> = input.routes.iter().rev().map(|r| &r.tag).collect();
    let chain = emit_chain(input);

    quote! {
        #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::default::Default)]
        pub struct Hook;

        #[allow(
            clippy::type_complexity,
            reason = "HList![…] of this plugin's routes plus prior plugins' R"
        )]
        impl<R> ::lariv_rs::http::RouteRegistrar<::lariv_rs::http::HttpCapability<R>> for Hook
        where
            R: ::frunk::hlist::HList + ::core::clone::Clone + ::lariv_rs::http::MountRoutes,
        {
            type Output = ::lariv_rs::http::HttpCapability<
                ::frunk::HList![
                    #(::lariv_rs::tag::Tagged<#rev_tags, ::lariv_rs::http::Route>,)*
                    ...R
                ],
            >;

            fn register_routes(
                self,
                http: ::lariv_rs::http::HttpCapability<R>,
            ) -> Self::Output {
                #chain
            }
        }
    }
}

fn emit_chain(input: &PluginRoutesInput) -> TokenStream2 {
    let mut acc = quote! { http };
    for route in &input.routes {
        let tag = &route.tag;
        let path_lit = &route.path;
        let handler = &route.handler;
        let method = match route.method {
            HttpMethod::Get => quote! { get },
            HttpMethod::Post => quote! { post },
        };
        acc = quote! {
            #acc.prepend::<#tag>(::lariv_rs::http::Route::#method(#path_lit, #handler))
        };
    }
    acc
}
