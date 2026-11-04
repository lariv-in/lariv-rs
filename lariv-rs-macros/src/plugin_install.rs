//! `define_plugin_install!` — plugin `install` function and `After*` type aliases.
//!
//! Expansions emit `::lariv_core::…` paths. `lariv-core` aliases itself so those paths
//! resolve when the macro is invoked from inside that crate.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Expr, Ident, Path, Token, Type, bracketed, parenthesized};

pub fn define_plugin_install(input: TokenStream) -> syn::Result<TokenStream> {
    let install: Install = syn::parse2(input)?;
    Ok(install.expand())
}

struct Install {
    plugin: Type,
    docs: Vec<Attribute>,
    steps: Vec<Step>,
    finish: Option<(Type, Expr)>,
}

enum Step {
    Export(Path),
    Apps(Path),
    RuneEnv(Path),
    Migrations(Path),
    Templates(Path),
    TemplatesGeneric { path: Path, idxs: Vec<Ident> },
    Slots(Path),
    Http(Path),
    State(Path),
    Seeds(Path),
    ServeStartup(Path),
    Commands(Path),
    Config { tag: Type, ty: Type },
    CapAttach { tag: Type, cap: Path, expr: Expr },
    CapHook { tag: Type, cap: Path, hook: Path },
}

impl Parse for Install {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let plugin_kw: Ident = input.parse()?;
        if plugin_kw != "plugin" {
            return Err(syn::Error::new(plugin_kw.span(), "expected `plugin:`"));
        }
        input.parse::<Token![:]>()?;
        let plugin: Type = input.parse()?;
        input.parse::<Token![;]>()?;
        let docs = Attribute::parse_outer(input)?;
        let steps_kw: Ident = input.parse()?;
        if steps_kw != "steps" {
            return Err(syn::Error::new(steps_kw.span(), "expected `steps:`"));
        }
        input.parse::<Token![:]>()?;
        let body;
        bracketed!(body in input);
        let steps = syn::punctuated::Punctuated::<Step, Token![,]>::parse_terminated(&body)?
            .into_iter()
            .collect();
        let finish = parse_finish(input)?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after `define_plugin_install!`"));
        }
        Ok(Self {
            plugin,
            docs,
            steps,
            finish,
        })
    }
}

fn parse_finish(input: ParseStream<'_>) -> syn::Result<Option<(Type, Expr)>> {
    if !input.peek(Token![;]) {
        return Ok(None);
    }
    input.parse::<Token![;]>()?;
    if input.is_empty() || !input.peek(Ident) {
        return Ok(None);
    }
    let finish_kw: Ident = input.parse()?;
    if finish_kw != "finish" {
        return Err(syn::Error::new(finish_kw.span(), "expected `finish:`"));
    }
    input.parse::<Token![:]>()?;
    let add_kw: Ident = input.parse()?;
    if add_kw != "add_capability" {
        return Err(syn::Error::new(add_kw.span(), "expected `add_capability`"));
    }
    let inner;
    parenthesized!(inner in input);
    let ty: Type = inner.parse()?;
    inner.parse::<Token![,]>()?;
    let expr: Expr = inner.parse()?;
    if !inner.is_empty() {
        return Err(inner.error("unexpected tokens in `add_capability`"));
    }
    if input.peek(Token![;]) {
        input.parse::<Token![;]>()?;
    }
    Ok(Some((ty, expr)))
}

impl Parse for Step {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let inner;
        parenthesized!(inner in input);
        let step = match name.to_string().as_str() {
            "export" => Step::Export(parse_path(&inner)?),
            "apps" => Step::Apps(parse_path(&inner)?),
            "rune_env" => Step::RuneEnv(parse_path(&inner)?),
            "migrations" => Step::Migrations(parse_path(&inner)?),
            "templates" => parse_templates(&inner)?,
            "slots" => Step::Slots(parse_path(&inner)?),
            "http" => Step::Http(parse_path(&inner)?),
            "state" => Step::State(parse_path(&inner)?),
            "seeds" => Step::Seeds(parse_path(&inner)?),
            "serve_startup" => Step::ServeStartup(parse_path(&inner)?),
            "commands" => Step::Commands(parse_path(&inner)?),
            "config" => {
                let tag: Type = inner.parse()?;
                inner.parse::<Token![,]>()?;
                let ty: Type = inner.parse()?;
                Step::Config { tag, ty }
            }
            "cap_attach" => {
                let tag: Type = inner.parse()?;
                inner.parse::<Token![,]>()?;
                let cap = Path::parse_mod_style(&inner)?;
                inner.parse::<Token![,]>()?;
                let expr: Expr = inner.parse()?;
                Step::CapAttach { tag, cap, expr }
            }
            "cap_hook" => {
                let tag: Type = inner.parse()?;
                inner.parse::<Token![,]>()?;
                let cap = Path::parse_mod_style(&inner)?;
                inner.parse::<Token![,]>()?;
                let hook = Path::parse_mod_style(&inner)?;
                Step::CapHook { tag, cap, hook }
            }
            other => {
                return Err(syn::Error::new(
                    name.span(),
                    format!("unknown install step `{other}`"),
                ));
            }
        };
        if !inner.is_empty() {
            return Err(inner.error("unexpected tokens in install step"));
        }
        Ok(step)
    }
}

fn parse_path(input: ParseStream<'_>) -> syn::Result<Path> {
    Path::parse_mod_style(input)
}

fn parse_templates(input: ParseStream<'_>) -> syn::Result<Step> {
    let path = Path::parse_mod_style(input)?;
    let mut idxs = Vec::new();
    while input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
        idxs.push(input.parse::<Ident>()?);
    }
    if idxs.is_empty() {
        Ok(Step::Templates(path))
    } else {
        Ok(Step::TemplatesGeneric { path, idxs })
    }
}

struct Acc {
    params: Vec<Ident>,
    prev: TokenStream,
    bounds: Vec<TokenStream>,
    calls: Vec<TokenStream>,
    proofs: Vec<Ident>,
    items: Vec<TokenStream>,
}

impl Acc {
    fn new() -> Self {
        let list = ident("L");
        Self {
            params: vec![list.clone()],
            prev: quote! { #list },
            bounds: Vec::new(),
            calls: Vec::new(),
            proofs: Vec::new(),
            items: Vec::new(),
        }
    }

    fn push_alias(&mut self, alias: Ident, extra: &[Ident], rhs: TokenStream) {
        let params = &self.params;
        self.items.push(quote! {
            type #alias<#(#params),* #(, #extra)*> = #rhs;
        });
        let mut all = self.params.clone();
        all.extend(extra.iter().cloned());
        self.prev = quote! { #alias<#(#all),*> };
        self.params = all;
    }
}

impl Install {
    fn expand(self) -> TokenStream {
        let mut acc = Acc::new();
        for step in &self.steps {
            step.apply(&self.plugin, &mut acc);
        }
        let params = &acc.params;
        let prev = &acc.prev;
        let bounds = &acc.bounds;
        let calls = &acc.calls;
        let proofs = &acc.proofs;
        let items = &acc.items;
        let docs = &self.docs;
        let plugin = &self.plugin;
        let allow = allow_type_complexity();
        match self.finish {
            None => quote! {
                #(#items)*
                type InstallOutput<#(#params),*> = ::lariv_core::app::App<#prev>;

                #(#docs)*
                #allow
                pub fn install<#(#params),* #(, #proofs)*>(
                    app: ::lariv_core::app::App<L>,
                ) -> InstallOutput<#(#params),*>
                where
                    #(#bounds)*
                {
                    app #(#calls)*
                }
            },
            Some((finish_cap, finish_expr)) => quote! {
                #(#items)*
                type InstallOutput<#(#params),*> =
                    ::lariv_core::app::App<::frunk::HCons<#finish_cap, #prev>>;

                #(#docs)*
                #allow
                pub fn install<#(#params),* #(, #proofs)*, Proof>(
                    app: ::lariv_core::app::App<L>,
                ) -> InstallOutput<#(#params),*>
                where
                    #(#bounds)*
                    #prev: ::frunk::hlist::HList
                        + ::lariv_core::traits::add::CapTagAbsent<#plugin, Proof>,
                {
                    app #(#calls)* .add_capability(#finish_expr)
                }
            },
        }
    }
}

impl Step {
    fn apply(&self, plugin: &Type, acc: &mut Acc) {
        match self {
            Step::Export(hook) => emit_two(
                acc,
                plugin,
                hook,
                "AfterExport",
                quote! { ::lariv_core::export::ExportTag },
                quote! { ::lariv_core::export::ExportCap },
                "ExportIdx",
                "ExportHooks",
            ),
            Step::Apps(hook) => emit_two(
                acc,
                plugin,
                hook,
                "AfterApps",
                quote! { ::lariv_core::apps::AppsTag },
                quote! { ::lariv_core::apps::AppsCap },
                "AppsIdx",
                "AppsHooks",
            ),
            Step::RuneEnv(hook) => emit_two(
                acc,
                plugin,
                hook,
                "AfterRuneEnv",
                quote! { ::lariv_core::rune_env::RuneEnvTag },
                quote! { ::lariv_core::rune_env::RuneEnvCap },
                "RuneEnvIdx",
                "RuneEnvHooks",
            ),
            Step::State(hook) => emit_two(
                acc,
                plugin,
                hook,
                "AfterStateHooks",
                quote! { ::lariv_core::hooks::StateHooksTag },
                quote! { ::lariv_core::hooks::StateHooksCap },
                "StateIdx",
                "StateHooks",
            ),
            Step::Seeds(hook) => emit_two(
                acc,
                plugin,
                hook,
                "AfterSeeds",
                quote! { ::lariv_core::hooks::SeedsTag },
                quote! { ::lariv_core::hooks::SeedsCap },
                "SeedsIdx",
                "SeedHooks",
            ),
            Step::ServeStartup(hook) => emit_two(
                acc,
                plugin,
                hook,
                "AfterServeStartup",
                quote! { ::lariv_core::hooks::ServeStartupsTag },
                quote! { ::lariv_core::hooks::ServeStartupsCap },
                "ServeIdx",
                "ServeHooks",
            ),
            Step::Migrations(hook) => emit_three(
                acc,
                plugin,
                hook,
                "AfterMigrations",
                quote! { ::lariv_core::migration::MigrationTag },
                quote! { ::lariv_core::migration::MigrationCap },
                "MigIdx",
                "MigHooks",
                "MigItems",
            ),
            Step::Templates(hook) => emit_three(
                acc,
                plugin,
                hook,
                "AfterTemplates",
                quote! { ::lariv_core::template::TemplateTag },
                quote! { ::lariv_core::template::TemplateCap },
                "TplIdx",
                "TplHooks",
                "TplItems",
            ),
            Step::Slots(hook) => emit_three(
                acc,
                plugin,
                hook,
                "AfterSlots",
                quote! { ::lariv_core::components::SlotTag },
                quote! { ::lariv_core::components::SlotCap },
                "SlotIdx",
                "SlotHooks",
                "SlotItems",
            ),
            Step::Commands(hook) => emit_three(
                acc,
                plugin,
                hook,
                "AfterCommands",
                quote! { ::lariv_core::command::CommandTag },
                quote! { ::lariv_core::command::CommandCap },
                "CmdIdx",
                "CmdHooks",
                "CmdItems",
            ),
            Step::TemplatesGeneric { path, idxs } => {
                emit_templates_generic(acc, plugin, path, idxs);
            }
            Step::Http(hook) => emit_http(acc, plugin, hook),
            Step::Config { tag, ty } => emit_config(acc, tag, ty),
            Step::CapAttach { tag, cap, expr } => emit_cap_attach(acc, tag, cap, expr),
            Step::CapHook { tag, cap, hook } => emit_cap_hook(acc, plugin, tag, cap, hook),
        }
    }
}

fn emit_two(
    acc: &mut Acc,
    plugin: &Type,
    hook: &Path,
    alias: &str,
    tag: TokenStream,
    cap: TokenStream,
    idx_name: &str,
    hooks_name: &str,
) {
    let idx = ident(idx_name);
    let hooks = ident(hooks_name);
    let prev = acc.prev.clone();
    let value = quote! { #cap<#hooks> };
    let new_cap =
        quote! { #cap<::frunk::HCons<::lariv_core::tag::Tagged<#plugin, #hook>, #hooks>> };
    push_replace(acc, &prev, &tag, &idx, &value, &new_cap, hook);
    let rhs = map_output(&prev, &tag, &new_cap, &idx);
    acc.push_alias(ident(alias), &[idx, hooks], rhs);
}

fn emit_three(
    acc: &mut Acc,
    plugin: &Type,
    hook: &Path,
    alias: &str,
    tag: TokenStream,
    cap: TokenStream,
    idx_name: &str,
    hooks_name: &str,
    items_name: &str,
) {
    let idx = ident(idx_name);
    let hooks = ident(hooks_name);
    let items = ident(items_name);
    let prev = acc.prev.clone();
    let value = quote! { #cap<#hooks, #items> };
    let new_cap = quote! {
        #cap<::frunk::HCons<::lariv_core::tag::Tagged<#plugin, #hook>, #hooks>, #items>
    };
    push_replace(acc, &prev, &tag, &idx, &value, &new_cap, hook);
    let rhs = map_output(&prev, &tag, &new_cap, &idx);
    acc.push_alias(ident(alias), &[idx, hooks, items], rhs);
}

fn push_replace(
    acc: &mut Acc,
    prev: &TokenStream,
    tag: &TokenStream,
    idx: &Ident,
    value: &TokenStream,
    new_cap: &TokenStream,
    hook: &Path,
) {
    acc.bounds.push(quote! {
        #prev: ::lariv_core::traits::get::GetByCapTag<#tag, #idx, Value = #value>,
    });
    acc.bounds.push(quote! {
        #prev: ::lariv_core::traits::replace::MapByCapTag<
            #tag,
            #new_cap,
            #idx,
            OldValue = #value,
        >,
    });
    acc.calls.push(quote! {
        .replace_capability::<#tag, #idx, _>(
            |cap: #value| { cap.add_hook(#hook::default()) },
        )
    });
}

fn emit_http(acc: &mut Acc, plugin: &Type, hook: &Path) {
    let idx = ident("HttpIdx");
    let hooks = ident("HttpHooks");
    let routes = ident("HttpRoutes");
    let prev = acc.prev.clone();
    let tag = quote! { ::lariv_core::http::HttpTag };
    let value = quote! {
        ::lariv_core::http::HttpCap<#hooks, ::lariv_core::http::HttpCapability<#routes>>
    };
    let new_cap = quote! {
        ::lariv_core::http::HttpCap<
            ::frunk::HCons<::lariv_core::tag::Tagged<#plugin, #hook>, #hooks>,
            ::lariv_core::http::HttpCapability<#routes>,
        >
    };
    push_replace(acc, &prev, &tag, &idx, &value, &new_cap, hook);
    let rhs = map_output(&prev, &tag, &new_cap, &idx);
    acc.push_alias(ident("AfterHttp"), &[idx, hooks, routes], rhs);
}

fn emit_config(acc: &mut Acc, cfg_tag: &Type, cfg_ty: &Type) {
    let idx = ident("CfgIdx");
    let configs = ident("Configs");
    let prev = acc.prev.clone();
    let tag = quote! { ::lariv_core::config::ConfigTag };
    let value = quote! { ::lariv_core::config::ConfigCap<::frunk::HNil, #configs> };
    let new_cap = quote! {
        ::lariv_core::config::ConfigCap<
            ::frunk::HNil,
            ::frunk::HCons<::lariv_core::tag::Tagged<#cfg_tag, #cfg_ty>, #configs>,
        >
    };
    acc.bounds.push(quote! {
        #prev: ::lariv_core::traits::get::GetByCapTag<#tag, #idx, Value = #value>,
    });
    acc.bounds.push(quote! {
        #prev: ::lariv_core::traits::replace::MapByCapTag<
            #tag,
            #new_cap,
            #idx,
            OldValue = #value,
        >,
    });
    acc.bounds.push(quote! {
        #configs: ::frunk::hlist::HList,
    });
    acc.calls.push(quote! {
        .replace_capability::<#tag, #idx, _>(
            |cap: #value| {
                cap.map_items(|items| ::frunk::HCons {
                    head: ::lariv_core::tag::Tagged::new(<#cfg_ty>::default()),
                    tail: items,
                })
            },
        )
    });
    let rhs = map_output(&prev, &tag, &new_cap, &idx);
    acc.push_alias(ident("AfterConfigs"), &[idx, configs], rhs);
}

fn emit_templates_generic(acc: &mut Acc, plugin: &Type, path: &Path, idxs: &[Ident]) {
    let idx = ident("TplIdx");
    let hooks = ident("TplHooks");
    let items = ident("TplItems");
    let prev = acc.prev.clone();
    let tag = quote! { ::lariv_core::template::TemplateTag };
    let value = quote! { ::lariv_core::template::TemplateCap<#hooks, #items> };
    let new_cap = quote! {
        ::lariv_core::template::TemplateCap<
            ::frunk::HCons<
                ::lariv_core::tag::Tagged<#plugin, TemplateHook<#(#idxs),*>>,
                #hooks,
            >,
            #items,
        >
    };
    acc.items.push(quote! {
        type TemplateHook<#(#idxs),*> = #path<#(#idxs),*>;
    });
    acc.bounds.push(quote! {
        #prev: ::lariv_core::traits::get::GetByCapTag<#tag, #idx, Value = #value>,
    });
    acc.bounds.push(quote! {
        #prev: ::lariv_core::traits::replace::MapByCapTag<
            #tag,
            #new_cap,
            #idx,
            OldValue = #value,
        >,
    });
    acc.calls.push(quote! {
        .replace_capability::<#tag, #idx, _>(
            |cap: #value| { cap.add_hook(<TemplateHook<#(#idxs),*>>::default()) },
        )
    });
    let rhs = map_output(&prev, &tag, &new_cap, &idx);
    let mut extra = vec![idx, hooks, items];
    extra.extend(idxs.iter().cloned());
    acc.push_alias(ident("AfterTemplates"), &extra, rhs);
}

fn emit_cap_attach(acc: &mut Acc, tag: &Type, cap: &Path, expr: &Expr) {
    let name = cap_name(cap);
    let alias = format_ident!("AfterCapAttach{}", name);
    let proof = format_ident!("CapAttachProof{}", name);
    let prev = acc.prev.clone();
    acc.bounds.push(quote! {
        #prev: ::frunk::hlist::HList
            + ::lariv_core::traits::add::CapTagAbsent<#tag, #proof>,
    });
    acc.bounds.push(quote! {
        #cap<::frunk::HNil>: ::lariv_core::capability::HasCapTag<Tag = #tag>,
    });
    acc.calls.push(quote! {
        .attach_cap(#expr)
    });
    acc.proofs.push(proof);
    acc.push_alias(
        alias,
        &[],
        quote! { ::frunk::HCons<#cap<::frunk::HNil>, #prev> },
    );
}

fn emit_cap_hook(acc: &mut Acc, plugin: &Type, tag: &Type, cap: &Path, hook: &Path) {
    let name = cap_name(cap);
    let cap_ty = format_ident!("CapTy{}", name);
    let after = format_ident!("AfterCapHook{}", name);
    let idx = format_ident!("CapIdx{}", name);
    let hooks = format_ident!("Hooks{}", name);
    let prev = acc.prev.clone();
    let value = quote! { #cap_ty<#hooks> };
    let new_cap = quote! {
        #cap_ty<::frunk::HCons<::lariv_core::tag::Tagged<#plugin, #hook>, #hooks>>
    };
    acc.items.push(quote! {
        type #cap_ty<#hooks> = #cap<#hooks>;
    });
    acc.bounds.push(quote! {
        #prev: ::lariv_core::traits::get::GetByCapTag<#tag, #idx, Value = #value>,
    });
    acc.bounds.push(quote! {
        #prev: ::lariv_core::traits::replace::MapByCapTag<
            #tag,
            #new_cap,
            #idx,
            OldValue = #value,
        >,
    });
    acc.calls.push(quote! {
        .replace_capability::<#tag, #idx, _>(
            |cap: #value| {
                <#value as ::lariv_core::capability::CapHookExt<#plugin, #hook>>::prepend_cap_hook(
                    cap,
                    <#hook as ::core::default::Default>::default(),
                )
            },
        )
    });
    let tag_ts = quote! { #tag };
    let rhs = map_output(&prev, &tag_ts, &new_cap, &idx);
    acc.push_alias(after, &[idx, hooks], rhs);
}

fn map_output(
    prev: &TokenStream,
    tag: &TokenStream,
    new_cap: &TokenStream,
    idx: &Ident,
) -> TokenStream {
    quote! { <#prev as ::lariv_core::traits::replace::MapByCapTag<#tag, #new_cap, #idx>>::Output }
}

fn cap_name(path: &Path) -> &Ident {
    &path
        .segments
        .last()
        .expect("capability path has a segment")
        .ident
}

fn ident(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

fn allow_type_complexity() -> TokenStream {
    quote! {
        #[allow(
            clippy::type_complexity,
            reason = "install return type is an HList capability stack; InstallOutput already aliases the shape"
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::define_plugin_install;

    #[test]
    fn expands_generic_templates_http_and_finish() -> syn::Result<()> {
        let rendered = define_plugin_install(
            syn::parse_str::<proc_macro2::TokenStream>(
                r#"
                plugin: SignupTag;
                /// Register signup.
                steps: [templates(templates::Hook, LoginIdx, UnauthIdx), http(routes::Hook)];
                finish: add_capability(DashboardStateCap, { CapStore::with_items(DashboardState) });
                "#,
            )?
            .into(),
        )?
        .to_string();
        assert!(rendered.contains("TemplateHook"), "{rendered}");
        assert!(rendered.contains("LoginIdx"), "{rendered}");
        assert!(rendered.contains("AfterHttp"), "{rendered}");
        assert!(rendered.contains("add_capability"), "{rendered}");
        assert!(rendered.contains("fn install"), "{rendered}");
        Ok(())
    }

    #[test]
    fn expands_cap_attach_and_hook() -> syn::Result<()> {
        let rendered = define_plugin_install(
            syn::parse_str::<proc_macro2::TokenStream>(
                r#"
                plugin: UsersTag;
                steps: [
                    cap_attach(AuthHooksTag, auth::AuthHooksCap, auth::AuthHooksCap::<frunk::HNil>::new()),
                    cap_hook(AuthHooksTag, auth::AuthHooksCap, routes::Hook),
                ]
                "#,
            )?
            .into(),
        )?
        .to_string();
        assert!(
            rendered.contains("AfterCapAttachAuthHooksCap"),
            "{rendered}"
        );
        assert!(
            rendered.contains("CapAttachProofAuthHooksCap"),
            "{rendered}"
        );
        assert!(rendered.contains("AfterCapHookAuthHooksCap"), "{rendered}");
        assert!(rendered.contains("attach_cap"), "{rendered}");
        assert!(rendered.contains("prepend_cap_hook"), "{rendered}");
        Ok(())
    }
}
