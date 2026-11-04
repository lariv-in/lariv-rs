//! Capability registrar macros moved out of `lariv-core`.
//!
//! Expansions emit `::lariv_core::…` paths. `lariv-core` aliases itself so those paths
//! resolve when the macros are invoked from inside that crate.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::ext::IdentExt;
use syn::{Expr, Ident, Path, Token, Type, bracketed, parse::Parse, parse::ParseStream};

pub fn apply_registrar_hook(input: TokenStream) -> syn::Result<TokenStream> {
    let input: ApplyRegistrarHook = syn::parse2(input)?;
    Ok(input.expand())
}

pub fn define_register_items(input: TokenStream) -> syn::Result<TokenStream> {
    let input: RegisterItems = syn::parse2(input)?;
    input.expand()
}

pub fn define_replace_templates(input: TokenStream) -> syn::Result<TokenStream> {
    let input: ReplaceTemplates = syn::parse2(input)?;
    Ok(input.expand())
}

pub fn define_passthrough_cap(input: TokenStream) -> syn::Result<TokenStream> {
    let input: PassthroughCap = syn::parse2(input)?;
    Ok(input.expand())
}

pub fn define_register_export(input: TokenStream) -> syn::Result<TokenStream> {
    let input: RegisterExport = syn::parse2(input)?;
    Ok(input.expand())
}

struct ApplyRegistrarHook {
    capability: Ident,
    trait_name: Ident,
    method: Ident,
    field: Ident,
    proof: Type,
    tag: Type,
}

impl Parse for ApplyRegistrarHook {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut capability = None;
        let mut trait_name = None;
        let mut method = None;
        let mut field = None;
        let mut proof = None;
        let mut tag = None;

        while !input.is_empty() {
            let key = Ident::parse_any(input)?;
            input.parse::<Token![:]>()?;
            if key == "capability" {
                assign(&mut capability, input.parse()?, &key)?;
            } else if key == "trait" {
                assign(&mut trait_name, input.parse()?, &key)?;
            } else if key == "method" {
                assign(&mut method, input.parse()?, &key)?;
            } else if key == "field" {
                assign(&mut field, input.parse()?, &key)?;
            } else if key == "proof" {
                assign(&mut proof, input.parse()?, &key)?;
            } else if key == "tag" {
                assign(&mut tag, input.parse()?, &key)?;
            } else {
                return Err(syn::Error::new(key.span(), "unknown field"));
            }
            if !input.is_empty() {
                input.parse::<Token![;]>()?;
            }
        }

        Ok(Self {
            capability: required(capability, "capability")?,
            trait_name: required(trait_name, "trait")?,
            method: required(method, "method")?,
            field: required(field, "field")?,
            proof: required(proof, "proof")?,
            tag: required(tag, "tag")?,
        })
    }
}

impl ApplyRegistrarHook {
    fn expand(&self) -> TokenStream {
        let cap = &self.capability;
        let trait_name = &self.trait_name;
        let method = &self.method;
        let field = &self.field;
        let proof = &self.proof;
        let tag = &self.tag;
        quote! {
            impl<Plugin, H, Tail, Items, TailOut, Out, TailProof>
                ::lariv_core::capability::ApplyHooks<Items, (TailProof, #proof)>
                for ::frunk::HCons<::lariv_core::tag::Tagged<Plugin, H>, Tail>
            where
                Items: ::lariv_core::capability::RegistrarItems,
                Tail: ::lariv_core::capability::ApplyHooks<Items, TailProof, Output = TailOut>,
                H: #trait_name<TailOut, Output = Out>,
            {
                type Output = Out;

                fn apply_hooks(self, items: Items) -> Self::Output {
                    let items = self.tail.apply_hooks(items);
                    H::#method(self.head.value, #cap { #field: items }).#field
                }
            }

            impl<Plugin, H, Tail, Items, TailOut, Out>
                ::lariv_core::capability::FoldRegistrarHooks<#tag, Items>
                for ::frunk::HCons<::lariv_core::tag::Tagged<Plugin, H>, Tail>
            where
                Items: ::lariv_core::capability::RegistrarItems,
                Tail: ::lariv_core::capability::FoldRegistrarHooks<#tag, Items, Output = TailOut>,
                H: #trait_name<TailOut, Output = Out>,
            {
                type Output = Out;

                fn fold_registrar_hooks(self, items: Items) -> Self::Output {
                    let items = self.tail.fold_registrar_hooks(items);
                    H::#method(self.head.value, #cap { #field: items }).#field
                }
            }
        }
    }
}

struct RegisterItems {
    capability: Ident,
    trait_name: Ident,
    method: Ident,
    wrapper: Option<Ident>,
    bounds: Vec<Path>,
    hook: Ident,
    items: ItemList,
}

enum ItemList {
    Empty,
    Two(Vec<TwoItem>),
    Three(Vec<ThreeItem>),
}

struct TwoItem {
    tag: Ident,
    value: Type,
}

struct ThreeItem {
    tag: Ident,
    kind: Type,
    value: Type,
}

impl Parse for RegisterItems {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut saw_plugin = false;
        let mut capability = None;
        let mut trait_name = None;
        let mut method = None;
        let mut wrapper = None;
        let mut bounds = None;
        let mut hook = None;
        let mut items = None;

        while !input.is_empty() {
            let key = Ident::parse_any(input)?;
            input.parse::<Token![:]>()?;
            if key == "plugin" {
                if saw_plugin {
                    return Err(syn::Error::new(key.span(), "duplicate `plugin`"));
                }
                let _: Type = input.parse()?;
                saw_plugin = true;
            } else if key == "capability" {
                assign(&mut capability, input.parse()?, &key)?;
            } else if key == "trait" {
                assign(&mut trait_name, input.parse()?, &key)?;
            } else if key == "method" {
                assign(&mut method, input.parse()?, &key)?;
            } else if key == "wrapper" {
                assign(&mut wrapper, input.parse()?, &key)?;
            } else if key == "bounds" {
                assign(&mut bounds, parse_paths(input)?, &key)?;
            } else if key == "hook" {
                assign(&mut hook, input.parse()?, &key)?;
            } else if key == "items" {
                assign(&mut items, parse_items(input)?, &key)?;
            } else {
                return Err(syn::Error::new(key.span(), "unknown field"));
            }
            if !input.is_empty() {
                input.parse::<Token![;]>()?;
            }
        }

        if !saw_plugin {
            return Err(syn::Error::new(Span::call_site(), "missing `plugin:`"));
        }
        Ok(Self {
            capability: required(capability, "capability")?,
            trait_name: required(trait_name, "trait")?,
            method: required(method, "method")?,
            wrapper,
            bounds: required(bounds, "bounds")?,
            hook: required(hook, "hook")?,
            items: required(items, "items")?,
        })
    }
}

impl RegisterItems {
    fn expand(&self) -> syn::Result<TokenStream> {
        let cap = &self.capability;
        let trait_name = &self.trait_name;
        let method = &self.method;
        let bounds = &self.bounds;
        let hook = &self.hook;
        let hook_struct = quote! {
            #[derive(Clone, Copy, Default)]
            pub struct #hook;
        };

        match &self.items {
            ItemList::Empty => Ok(quote! {
                #hook_struct

                impl<T: ::frunk::hlist::HList #(+ #bounds)*> #trait_name<T> for #hook {
                    type Output = T;

                    fn #method(self, cap: #cap<T>) -> #cap<Self::Output> {
                        cap
                    }
                }
            }),
            ItemList::Two(items) => {
                let wrapper = self.wrapper.as_ref().ok_or_else(|| {
                    syn::Error::new(
                        Span::call_site(),
                        "missing `wrapper:` for `Tag => Value` items",
                    )
                })?;
                let tags: Vec<&Ident> = items.iter().map(|item| &item.tag).collect();
                let rev_tags: Vec<&Ident> = tags.iter().rev().copied().collect();
                let rev_values: Vec<&Type> = items.iter().rev().map(|item| &item.value).collect();
                let fwd_tags: Vec<&Ident> = tags;
                let fwd_values: Vec<&Type> = items.iter().map(|item| &item.value).collect();
                Ok(quote! {
                    #(pub struct #rev_tags;)*
                    #hook_struct

                    #[allow(
                        clippy::type_complexity,
                        reason = "HList![…] of registered items plus prior plugins' T"
                    )]
                    impl<T: ::frunk::hlist::HList #(+ #bounds)*> #trait_name<T> for #hook {
                        type Output = ::frunk::HList![
                            #(::lariv_core::tag::Tagged<#rev_tags, #wrapper<#rev_values>>),*,
                            ...T
                        ];

                        fn #method(self, cap: #cap<T>) -> #cap<Self::Output> {
                            cap
                                #(.add::<#fwd_tags, #fwd_values>())*
                        }
                    }
                })
            }
            ItemList::Three(items) => {
                let wrapper = self.wrapper.as_ref().ok_or_else(|| {
                    syn::Error::new(
                        Span::call_site(),
                        "missing `wrapper:` for `Tag, Kind => Value` items",
                    )
                })?;
                let tags: Vec<&Ident> = items.iter().map(|item| &item.tag).collect();
                let rev_tags: Vec<&Ident> = tags.iter().rev().copied().collect();
                let rev_kinds: Vec<&Type> = items.iter().rev().map(|item| &item.kind).collect();
                let rev_values: Vec<&Type> = items.iter().rev().map(|item| &item.value).collect();
                let fwd_tags: Vec<&Ident> = tags;
                let fwd_kinds: Vec<&Type> = items.iter().map(|item| &item.kind).collect();
                let fwd_values: Vec<&Type> = items.iter().map(|item| &item.value).collect();
                Ok(quote! {
                    #(pub struct #rev_tags;)*
                    #hook_struct

                    #[allow(
                        clippy::type_complexity,
                        reason = "HList![…] of registered items plus prior plugins' T"
                    )]
                    impl<T: ::frunk::hlist::HList #(+ #bounds)*> #trait_name<T> for #hook {
                        type Output = ::frunk::HList![
                            #(::lariv_core::tag::Tagged<
                                #rev_tags,
                                #wrapper<#rev_kinds, #rev_values>
                            >),*,
                            ...T
                        ];

                        fn #method(self, cap: #cap<T>) -> #cap<Self::Output> {
                            cap
                                #(.add::<#fwd_tags, #fwd_kinds, #fwd_values>())*
                        }
                    }
                })
            }
        }
    }
}

struct ReplaceTemplates {
    hook: Ident,
    bounds: Vec<Path>,
    replaces: Vec<ReplaceItem>,
    adds: Vec<AddItem>,
}

struct ReplaceItem {
    idx: Ident,
    tag: Ident,
    value: Type,
}

struct AddItem {
    tag: Ident,
    value: Type,
}

impl Parse for ReplaceTemplates {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut hook = None;
        let mut bounds = None;
        let mut replaces = None;
        let mut adds = None;

        while !input.is_empty() {
            let key = Ident::parse_any(input)?;
            input.parse::<Token![:]>()?;
            if key == "hook" {
                assign(&mut hook, input.parse()?, &key)?;
            } else if key == "bounds" {
                assign(&mut bounds, parse_paths(input)?, &key)?;
            } else if key == "replaces" {
                assign(&mut replaces, parse_replaces(input)?, &key)?;
            } else if key == "adds" {
                assign(&mut adds, parse_adds(input)?, &key)?;
            } else {
                return Err(syn::Error::new(key.span(), "unknown field"));
            }
            if !input.is_empty() {
                input.parse::<Token![;]>()?;
            }
        }

        Ok(Self {
            hook: required(hook, "hook")?,
            bounds: required(bounds, "bounds")?,
            replaces: required(replaces, "replaces")?,
            adds: required(adds, "adds")?,
        })
    }
}

impl ReplaceTemplates {
    fn expand(&self) -> TokenStream {
        let hook = &self.hook;
        let bounds = &self.bounds;
        let mut acc = quote!(T);
        let mut where_preds = Vec::new();
        let mut idxs = Vec::new();
        let mut rep_tags = Vec::new();
        let mut rep_values = Vec::new();
        for item in &self.replaces {
            let prev = acc.clone();
            let idx = &item.idx;
            let tag = &item.tag;
            let value = &item.value;
            where_preds.push(quote! {
                #prev: ::lariv_core::traits::get::IndexOfTemplateTag<#tag, #idx>,
                #prev: ::lariv_core::traits::replace::MapByTag<
                    #tag,
                    ::lariv_core::template::TemplateOf<#value>,
                    #idx
                >,
            });
            acc = quote! {
                ::lariv_core::traits::get::ReplaceTemplateAtTag<
                    #prev,
                    #tag,
                    ::lariv_core::template::TemplateOf<#value>,
                    #idx
                >
            };
            idxs.push(idx);
            rep_tags.push(tag);
            rep_values.push(value);
        }

        let rev_add_tags: Vec<&Ident> = self.adds.iter().rev().map(|item| &item.tag).collect();
        let rev_add_values: Vec<&Type> = self.adds.iter().rev().map(|item| &item.value).collect();
        let fwd_add_tags: Vec<&Ident> = self.adds.iter().map(|item| &item.tag).collect();
        let fwd_add_values: Vec<&Type> = self.adds.iter().map(|item| &item.value).collect();

        let output = if self.adds.is_empty() {
            acc.clone()
        } else {
            quote! {
                ::frunk::HList![
                    #(::lariv_core::tag::Tagged<
                        #rev_add_tags,
                        ::lariv_core::template::TemplateOf<#rev_add_values>
                    >),*,
                    ... #acc
                ]
            }
        };

        let allow = if self.adds.is_empty() {
            TokenStream::new()
        } else {
            quote! {
                #[allow(
                    clippy::type_complexity,
                    reason = "HList of prepended templates plus replaced chain"
                )]
            }
        };

        quote! {
            #[derive(Clone, Copy, Default)]
            pub struct #hook;

            #allow
            impl<T: ::frunk::hlist::HList #(+ #bounds)* #(, #idxs)*>
                ::lariv_core::template::TemplateRegistrar<T> for #hook
            where
                #acc: ::frunk::hlist::HList,
                #(#where_preds)*
            {
                type Output = #output;

                fn register_templates(
                    self,
                    cap: ::lariv_core::template::TemplateCapability<T>,
                ) -> ::lariv_core::template::TemplateCapability<Self::Output> {
                    cap
                        #(.replace_template_tag::<
                            #rep_tags,
                            ::lariv_core::template::TemplateOf<#rep_values>,
                            _,
                        >(|_| ::lariv_core::template::TemplateOf::new()))*
                        #(.add::<#fwd_add_tags, #fwd_add_values>())*
                }
            }
        }
    }
}

struct RegisterExport {
    plugin: Type,
    table: Expr,
    model: Expr,
    columns: Vec<Expr>,
    deps: Option<Vec<Expr>>,
}

impl Parse for RegisterExport {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut plugin = None;
        let mut table = None;
        let mut model = None;
        let mut columns = None;
        let mut deps = None;

        while !input.is_empty() {
            let key = Ident::parse_any(input)?;
            input.parse::<Token![:]>()?;
            if key == "plugin" {
                assign(&mut plugin, input.parse()?, &key)?;
            } else if key == "table" {
                assign(&mut table, input.parse()?, &key)?;
            } else if key == "model" {
                assign(&mut model, input.parse()?, &key)?;
            } else if key == "columns" {
                assign(&mut columns, parse_expr_list(input)?, &key)?;
            } else if key == "deps" {
                assign(&mut deps, parse_expr_list(input)?, &key)?;
            } else {
                return Err(syn::Error::new(key.span(), "unknown field"));
            }
            if !input.is_empty() {
                input.parse::<Token![;]>()?;
            }
        }

        Ok(Self {
            plugin: required(plugin, "plugin")?,
            table: required(table, "table")?,
            model: required(model, "model")?,
            columns: required(columns, "columns")?,
            deps,
        })
    }
}

impl RegisterExport {
    fn expand(&self) -> TokenStream {
        // Accepted so call sites stay stable; the generated hook does not name the plugin tag.
        let _plugin = &self.plugin;
        let table = &self.table;
        let model = &self.model;
        let columns = &self.columns;
        let with_deps = self.deps.as_ref().map(|deps| {
            quote! {
                .with_deps(vec![#(::std::convert::Into::into(#deps)),*])
            }
        });
        quote! {
            #[derive(Clone, Copy, Default)]
            pub struct ExportHook;

            impl ::lariv_core::export::ExportRegistrar for ExportHook {
                fn register_export(
                    self,
                    export: ::lariv_core::export::ExportCapability,
                ) -> ::lariv_core::export::ExportCapability {
                    export.register(
                        ::lariv_core::export::ExportTable::new(
                            #table,
                            #model,
                            vec![#(::std::convert::Into::into(#columns)),*],
                        )
                        #with_deps
                    )
                }
            }
        }
    }
}

struct PassthroughCap {
    cap: Ident,
    tag: Type,
    value: Type,
}

impl Parse for PassthroughCap {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let cap = input.parse()?;
        input.parse::<Token![,]>()?;
        let tag = input.parse()?;
        input.parse::<Token![,]>()?;
        let value = input.parse()?;
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        Ok(Self { cap, tag, value })
    }
}

impl PassthroughCap {
    fn expand(&self) -> TokenStream {
        let cap = &self.cap;
        let tag = &self.tag;
        let value = &self.value;
        quote! {
            pub type #cap = ::lariv_core::capability::CapStore<#tag, ::frunk::HNil, #value>;

            impl ::lariv_core::capability::PassthroughCapTag for #tag {}
        }
    }
}

fn parse_expr_list(input: ParseStream<'_>) -> syn::Result<Vec<Expr>> {
    let content;
    bracketed!(content in input);
    let punctuated = syn::punctuated::Punctuated::<Expr, Token![,]>::parse_terminated(&content)?;
    Ok(punctuated.into_iter().collect())
}

fn parse_paths(input: ParseStream<'_>) -> syn::Result<Vec<Path>> {
    let content;
    bracketed!(content in input);
    let punctuated = syn::punctuated::Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
    Ok(punctuated.into_iter().collect())
}

fn parse_items(input: ParseStream<'_>) -> syn::Result<ItemList> {
    let content;
    bracketed!(content in input);
    if content.is_empty() {
        return Ok(ItemList::Empty);
    }
    let mut two = Vec::new();
    let mut three = Vec::new();
    while !content.is_empty() {
        let _idx: Ident = content.parse()?;
        content.parse::<Token![:]>()?;
        let tag: Ident = content.parse()?;
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
            let kind: Type = content.parse()?;
            content.parse::<Token![=>]>()?;
            let value: Type = content.parse()?;
            if !two.is_empty() {
                return Err(syn::Error::new(
                    tag.span(),
                    "items must be all `Tag => Value` or all `Tag, Kind => Value`",
                ));
            }
            three.push(ThreeItem { tag, kind, value });
        } else {
            content.parse::<Token![=>]>()?;
            let value: Type = content.parse()?;
            if !three.is_empty() {
                return Err(syn::Error::new(
                    tag.span(),
                    "items must be all `Tag => Value` or all `Tag, Kind => Value`",
                ));
            }
            two.push(TwoItem { tag, value });
        }
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    if three.is_empty() {
        Ok(ItemList::Two(two))
    } else {
        Ok(ItemList::Three(three))
    }
}

fn parse_replaces(input: ParseStream<'_>) -> syn::Result<Vec<ReplaceItem>> {
    let content;
    bracketed!(content in input);
    let mut items = Vec::new();
    while !content.is_empty() {
        let idx = content.parse()?;
        content.parse::<Token![:]>()?;
        let tag = content.parse()?;
        content.parse::<Token![=>]>()?;
        let value = content.parse()?;
        items.push(ReplaceItem { idx, tag, value });
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(items)
}

fn parse_adds(input: ParseStream<'_>) -> syn::Result<Vec<AddItem>> {
    let content;
    bracketed!(content in input);
    let mut items = Vec::new();
    while !content.is_empty() {
        let tag = content.parse()?;
        content.parse::<Token![=>]>()?;
        let value = content.parse()?;
        items.push(AddItem { tag, value });
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(items)
}

fn assign<T>(slot: &mut Option<T>, value: T, key: &Ident) -> syn::Result<()> {
    if slot.is_some() {
        return Err(syn::Error::new(key.span(), format!("duplicate `{}`", key)));
    }
    *slot = Some(value);
    Ok(())
}

fn required<T>(value: Option<T>, name: &str) -> syn::Result<T> {
    value.ok_or_else(|| syn::Error::new(Span::call_site(), format!("missing `{name}:`")))
}

#[cfg(test)]
mod tests {
    use super::{RegisterExport, ReplaceTemplates};

    #[test]
    fn register_export_expands_table_and_optional_deps() -> syn::Result<()> {
        let parsed: RegisterExport = syn::parse_str(
            r#"plugin: UsersTag; table: "users"; model: "User"; columns: ["id", "email"]; deps: ["clients"];"#,
        )?;
        let rendered = parsed.expand().to_string();
        assert!(rendered.contains("ExportHook"), "{rendered}");
        assert!(rendered.contains("with_deps"), "{rendered}");
        assert!(rendered.contains("\"clients\""), "{rendered}");
        Ok(())
    }

    #[test]
    fn replace_templates_expands_replaces_and_adds() -> syn::Result<()> {
        let parsed: ReplaceTemplates = syn::parse_str(
            "hook: Hook; bounds: [Clone]; replaces: [LoginIdx: UsersLoginPageTag => LoginPage]; adds: [ExtraTag => ExtraPage];",
        )?;
        let rendered = parsed.expand().to_string();
        assert!(rendered.contains("ReplaceTemplateAtTag"));
        assert!(rendered.contains("UsersLoginPageTag"));
        assert!(rendered.contains("ExtraPage"), "{rendered}");
        assert!(rendered.contains("replace_template_tag"), "{rendered}");
        Ok(())
    }
}
