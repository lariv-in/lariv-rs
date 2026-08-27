//! `#[html_form]` — inject serde attrs + emit [`HtmlForm`] (structs and tagged enums).

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Error, Fields, GenericArgument, Ident, Meta, PathArguments, Result, Type,
    parse_macro_input, spanned::Spanned,
};

pub fn html_form_attr(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = match syn::parse::<HtmlFormArgs>(attr) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };
    let input = parse_macro_input!(item as DeriveInput);
    match expand(&input, &args) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

struct HtmlFormArgs {
    default: bool,
    no_debug: bool,
    tag: Option<String>,
}

impl syn::parse::Parse for HtmlFormArgs {
    fn parse(input: syn::parse::ParseStream<'_>) -> Result<Self> {
        let mut default = false;
        let mut no_debug = false;
        let mut tag = None;
        if input.is_empty() {
            return Ok(Self {
                default,
                no_debug,
                tag,
            });
        }
        let metas = input.parse_terminated(Meta::parse, syn::Token![,])?;
        for meta in metas {
            if meta.path().is_ident("default") {
                default = true;
            } else if meta.path().is_ident("no_debug") {
                no_debug = true;
            } else if meta.path().is_ident("tag") {
                let Meta::NameValue(nv) = &meta else {
                    return Err(Error::new_spanned(meta, "tag expects tag = \"Kind\""));
                };
                let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                else {
                    return Err(Error::new_spanned(meta, "tag expects a string"));
                };
                tag = Some(s.value());
            } else {
                return Err(Error::new_spanned(meta, "unknown html_form option"));
            }
        }
        Ok(Self {
            default,
            no_debug,
            tag,
        })
    }
}

fn expand(input: &DeriveInput, args: &HtmlFormArgs) -> Result<proc_macro2::TokenStream> {
    match &input.data {
        Data::Struct(_) => expand_struct(input, args),
        Data::Enum(_) => expand_enum(input, args),
        Data::Union(_) => Err(Error::new(
            input.span(),
            "html_form does not support unions",
        )),
    }
}

// --- shared field helpers ---------------------------------------------------

struct PreparedField {
    ident: Ident,
    vis: syn::Visibility,
    ty: Type,
    html_name: String,
    label: String,
    form: FormAttrs,
    widget: syn::Path,
    is_kind: bool,
    upload_kind: UploadKind,
    skip_deser: bool,
    extra_attrs: Vec<syn::Attribute>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UploadKind {
    None,
    One,
    Optional,
    Many,
}

fn prepare_field(field: &syn::Field) -> Result<PreparedField> {
    let ident = field
        .ident
        .clone()
        .ok_or_else(|| Error::new(field.span(), "expected named field"))?;
    let form = parse_form_attrs(field)?;
    let widget = form.widget.clone().ok_or_else(|| {
        Error::new(
            field.span(),
            "#[form(widget = ...)] is required on every field",
        )
    })?;
    let rust_name = ident.to_string();
    let html_name = form
        .name
        .clone()
        .unwrap_or_else(|| to_pascal_html_name(&rust_name));
    let label = form
        .label
        .clone()
        .unwrap_or_else(|| humanize_label(&html_name));
    let is_kind = widget_is_kind(&widget);
    let upload_kind = classify_upload(&field.ty);
    let is_unit = matches!(&field.ty, Type::Tuple(t) if t.elems.is_empty());
    let is_section = widget_is_section(&widget);
    let skip_deser =
        is_unit || is_section || form.skip_deser || upload_kind != UploadKind::None || is_kind;
    let extra_attrs: Vec<_> = field
        .attrs
        .iter()
        .filter(|a| !a.path().is_ident("form") && !a.path().is_ident("serde"))
        .cloned()
        .collect();
    Ok(PreparedField {
        ident,
        vis: field.vis.clone(),
        ty: field.ty.clone(),
        html_name,
        label,
        form,
        widget,
        is_kind,
        upload_kind,
        skip_deser,
        extra_attrs,
    })
}

fn widget_is_m2m(widget: &syn::Path) -> bool {
    widget
        .segments
        .last()
        .is_some_and(|s| s.ident == "ManyToMany")
}

fn widget_is_fk(widget: &syn::Path) -> bool {
    widget
        .segments
        .last()
        .is_some_and(|s| s.ident == "ForeignKey")
}

fn field_spec_tokens(f: &PreparedField) -> proc_macro2::TokenStream {
    let html_name = &f.html_name;
    let label = &f.label;
    let required = f.form.required;
    let multiple = f.form.multiple || f.upload_kind == UploadKind::Many;
    let url = opts_str(f.form.url.as_deref());
    let swap_key = opts_str(f.form.swap_key.as_deref());
    let display = opts_str(f.form.display.as_deref());
    let error = opts_str(f.form.error.as_deref());
    let choices = opts_str(f.form.choices.as_deref());
    let when = opts_str(f.form.when.as_deref());
    let required_unless = opts_str(f.form.required_unless.as_deref());
    let model = opts_str(f.form.model.as_deref());
    let show = opts_str(f.form.show.as_deref());
    let placeholder = opts_str(f.form.placeholder.as_deref());
    let accept = opts_str(f.form.accept.as_deref());
    let row = opts_str(f.form.row.as_deref());
    let hint = match &f.form.hint {
        Some(HintExpr::Lit(s)) => quote! { ::core::option::Option::Some(#s) },
        Some(HintExpr::Path(p)) => quote! { ::core::option::Option::Some(#p) },
        None => quote! { ::core::option::Option::None },
    };
    let rows = match f.form.rows {
        Some(n) => quote! { ::core::option::Option::Some(#n) },
        None => quote! { ::core::option::Option::None },
    };
    let render = if f.is_kind {
        let ty = &f.ty;
        quote! {
            |ctx, field| ::lariv_rs::html_form::render_kind::<#ty>(ctx, field)
        }
    } else if let Some(route) = &f.form.route {
        let widget = &f.widget;
        if widget_is_fk(widget) {
            quote! {
                |ctx, field| {
                    let default_url = #route.url();
                    let url = ctx.url_of(field.spec);
                    let url = if url.is_empty() { default_url.as_str() } else { url };
                    let display_key = field.spec.display_key.unwrap_or(field.name);
                    let ph = field.spec.placeholder.unwrap_or("Select...");
                    ::lariv_rs::components::input_foreign_key(
                        ::lariv_rs::components::InputForeignKey {
                            label: field.label,
                            name: field.name,
                            value: field.value,
                            display: ctx.display_of(display_key),
                            placeholder: ph,
                            url,
                            uid: field.spec.swap_key.unwrap_or(""),
                            required: field.required,
                            ..Default::default()
                        },
                    )
                }
            }
        } else if widget_is_m2m(widget) {
            quote! {
                |ctx, field| {
                    let default_url = #route.url();
                    let url = ctx.url_of(field.spec);
                    let url = if url.is_empty() { default_url.as_str() } else { url };
                    let ph = field.spec.placeholder.unwrap_or("Select...");
                    let attrs = match field.spec.swap_key {
                        Some(id) => ::lariv_rs::components::HtmlAttrs::new().set("id", id),
                        None => ::lariv_rs::components::HtmlAttrs::new(),
                    };
                    ::lariv_rs::components::input_many_to_many(
                        ::lariv_rs::components::InputManyToMany {
                            label: field.label,
                            name: field.name,
                            items: ctx.m2m_of(field.name),
                            placeholder: ph,
                            url,
                            attrs,
                            ..Default::default()
                        },
                    )
                }
            }
        } else {
            let widget = &f.widget;
            quote! { <#widget as ::lariv_rs::html_form::FormWidget>::render }
        }
    } else {
        let widget = &f.widget;
        quote! { <#widget as ::lariv_rs::html_form::FormWidget>::render }
    };
    quote! {
        ::lariv_rs::html_form::FieldSpec {
            name: #html_name,
            label: #label,
            required: #required,
            row: #row,
            when: #when,
            required_unless: #required_unless,
            model: #model,
            show: #show,
            url: #url,
            swap_key: #swap_key,
            display_key: #display,
            error_key: #error,
            choices_key: #choices,
            placeholder: #placeholder,
            hint: #hint,
            rows: #rows,
            multiple: #multiple,
            accept: #accept,
            render: #render,
        }
    }
}

fn serde_attrs_for_field(f: &PreparedField) -> proc_macro2::TokenStream {
    let html_name = &f.html_name;
    let rust_name = f.ident.to_string();
    let ty = &f.ty;
    if f.skip_deser {
        return quote! { skip };
    }
    let mut attrs = vec![
        quote! { rename = #html_name },
        quote! { alias = #rust_name },
    ];
    if is_option(ty) {
        attrs.push(quote! { default });
        if is_option_integer(ty) {
            attrs.push(quote! {
                deserialize_with = "::lariv_rs::html_form::empty_str_as_none"
            });
        }
    } else if is_integer(ty) {
        attrs.push(quote! { default });
        attrs.push(quote! {
            deserialize_with = "::lariv_rs::html_form::empty_str_as_i64"
        });
    } else if is_vec_integer(ty) {
        attrs.push(quote! { default });
        attrs.push(quote! {
            deserialize_with = "::lariv_rs::html_form::form_vec_i64"
        });
    } else if is_vec_string(ty) {
        attrs.push(quote! { default });
        attrs.push(quote! {
            deserialize_with = "::lariv_rs::html_form::form_vec_string"
        });
    } else if is_bool(ty) {
        attrs.push(quote! { default });
        attrs.push(quote! {
            deserialize_with = "::lariv_rs::html_form::form_checkbox_bool"
        });
    } else if matches_defaultable(ty) {
        attrs.push(quote! { default });
    }
    quote! { #(#attrs),* }
}

fn submit_ty_for_field(f: &PreparedField) -> proc_macro2::TokenStream {
    match f.upload_kind {
        UploadKind::One => quote! { ::lariv_rs::html_form::UploadedFile },
        UploadKind::Optional => {
            quote! { ::core::option::Option<::lariv_rs::html_form::UploadedFile> }
        }
        UploadKind::Many => quote! { ::std::vec::Vec<::lariv_rs::html_form::UploadedFile> },
        UploadKind::None if f.is_kind => {
            let ty = &f.ty;
            quote! { <#ty as ::lariv_rs::html_form::HtmlForm>::Submit }
        }
        UploadKind::None => {
            let ty = &f.ty;
            quote! { #ty }
        }
    }
}

fn collect_file_names(fields: &[PreparedField]) -> (Vec<String>, Vec<String>) {
    let mut ones = Vec::new();
    let mut manys = Vec::new();
    for f in fields {
        match f.upload_kind {
            UploadKind::One | UploadKind::Optional => ones.push(f.html_name.clone()),
            UploadKind::Many => manys.push(f.html_name.clone()),
            UploadKind::None => {}
        }
    }
    (ones, manys)
}

// --- struct -----------------------------------------------------------------

fn expand_struct(input: &DeriveInput, args: &HtmlFormArgs) -> Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let Data::Struct(data) = &input.data else {
        unreachable!()
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(Error::new(input.span(), "html_form requires named fields"));
    };

    let prepared: Vec<PreparedField> = fields
        .named
        .iter()
        .map(prepare_field)
        .collect::<Result<_>>()?;

    let needs_submit = prepared
        .iter()
        .any(|f| f.upload_kind != UploadKind::None || f.is_kind);
    let submit_name = format_ident!("{name}Submit");

    let mut out_fields = Vec::new();
    let mut submit_fields = Vec::new();
    let mut specs = Vec::new();

    for f in &prepared {
        let serde_attrs = serde_attrs_for_field(f);
        let ident = &f.ident;
        let ty = &f.ty;
        let field_vis = &f.vis;
        let extra = &f.extra_attrs;
        out_fields.push(quote! {
            #(#extra)*
            #[serde(#serde_attrs)]
            #field_vis #ident: #ty
        });
        specs.push(field_spec_tokens(f));

        let submit_ty = submit_ty_for_field(f);
        submit_fields.push(quote! {
            #field_vis #ident: #submit_ty
        });
    }

    let (file_ones, file_manys) = collect_file_names(&prepared);
    let file_ones_lit = &file_ones;
    let file_manys_lit = &file_manys;

    let field_enum_name = format_ident!("{name}Field");
    let flag_enum_name = format_ident!("{name}Flag");
    let (field_enum, field_impl) = emit_field_key_enum(&field_enum_name, &prepared);
    let (flag_enum, flag_impl) = emit_flag_key_enum(&flag_enum_name, &prepared);

    // Build assemble_submit body
    let assemble = assemble_struct_submit(&prepared, needs_submit.then_some(&submit_name))?;

    let mut derives = vec![quote! { ::serde::Deserialize }];
    if !args.no_debug {
        derives.insert(0, quote! { ::core::fmt::Debug });
    }
    if args.default {
        derives.push(quote! { ::core::default::Default });
    }

    let struct_attrs: Vec<_> = input
        .attrs
        .iter()
        .filter(|a| !a.path().is_ident("derive"))
        .collect();

    let submit_def = if needs_submit {
        quote! {
            #[derive(Debug)]
            #vis struct #submit_name {
                #(#submit_fields),*
            }
        }
    } else {
        quote! {}
    };

    let submit_ty = if needs_submit {
        quote! { #submit_name }
    } else {
        quote! { Self }
    };

    Ok(quote! {
        #(#struct_attrs)*
        #[derive(#(#derives),*)]
        #vis struct #name #generics {
            #(#out_fields),*
        }

        #submit_def

        #field_enum
        #field_impl

        #flag_enum
        #flag_impl

        impl #impl_generics ::lariv_rs::html_form::HtmlForm for #name #ty_generics #where_clause {
            type Field = #field_enum_name;
            type Flag = #flag_enum_name;
            type Submit = #submit_ty;

            fn field_specs() -> &'static [::lariv_rs::html_form::FieldSpec] {
                &[#(#specs),*]
            }

            fn file_field_names() -> &'static [&'static str] {
                &[#(#file_ones_lit),*]
            }

            fn multi_file_field_names() -> &'static [&'static str] {
                &[#(#file_manys_lit),*]
            }

            fn assemble_submit(
                mut parts: ::lariv_rs::html_form::MultipartParts,
            ) -> ::core::result::Result<Self::Submit, ::lariv_rs::html_form::FormError> {
                #assemble
            }
        }
    })
}

fn assemble_struct_submit(
    fields: &[PreparedField],
    submit_name: Option<&Ident>,
) -> Result<proc_macro2::TokenStream> {
    // Wire type: only non-skip, non-kind, non-upload fields for serde JSON map deser,
    // plus kind discriminant strings.
    let mut wire_fields = Vec::new();
    let mut wire_idents = Vec::new();
    for f in fields {
        if f.upload_kind != UploadKind::None {
            continue;
        }
        if f.is_kind {
            let ident = &f.ident;
            let html = &f.html_name;
            let rust = f.ident.to_string();
            wire_fields.push(quote! {
                #[serde(rename = #html, alias = #rust, default)]
                #ident: ::std::string::String
            });
            wire_idents.push(ident.clone());
            continue;
        }
        if f.skip_deser {
            continue;
        }
        let ident = &f.ident;
        let ty = &f.ty;
        let serde_attrs = serde_attrs_for_field(f);
        wire_fields.push(quote! {
            #[serde(#serde_attrs)]
            #ident: #ty
        });
        wire_idents.push(ident.clone());
    }

    let mut assign = Vec::new();
    for f in fields {
        let ident = &f.ident;
        let html = &f.html_name;
        match f.upload_kind {
            UploadKind::One => {
                assign.push(quote! {
                    #ident: parts.files.remove(#html).ok_or_else(|| {
                        ::lariv_rs::html_form::FormError::Validation(
                            ::std::format!("{} is required", #html),
                        )
                    })?
                });
            }
            UploadKind::Optional => {
                assign.push(quote! {
                    #ident: parts.files.remove(#html)
                });
            }
            UploadKind::Many => {
                assign.push(quote! {
                    #ident: parts.file_lists.remove(#html).unwrap_or_default()
                });
            }
            UploadKind::None if f.is_kind => {
                let ty = &f.ty;
                assign.push(quote! {
                    #ident: {
                        let tag = <#ty as ::lariv_rs::html_form::HtmlKind>::kind_tag();
                        let mut kind_parts = ::lariv_rs::html_form::MultipartParts {
                            text: ::lariv_rs::html_form::UrlencodedFields::default(),
                            files: ::std::collections::HashMap::new(),
                            file_lists: ::std::collections::HashMap::new(),
                        };
                        let disc = parts
                            .text
                            .get_first(tag)
                            .map(|s| s.to_string())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| wire.#ident.clone());
                        if !disc.is_empty() {
                            kind_parts.text.push(tag, disc);
                        }
                        for n in <#ty as ::lariv_rs::html_form::HtmlForm>::file_field_names() {
                            if let Some(file) = parts.files.remove(*n) {
                                kind_parts.files.insert((*n).to_string(), file);
                            }
                        }
                        for n in <#ty as ::lariv_rs::html_form::HtmlForm>::multi_file_field_names()
                        {
                            if let Some(list) = parts.file_lists.remove(*n) {
                                kind_parts.file_lists.insert((*n).to_string(), list);
                            }
                        }
                        <#ty as ::lariv_rs::html_form::HtmlForm>::assemble_submit(kind_parts)?
                    }
                });
            }
            UploadKind::None if f.skip_deser => {
                assign.push(quote! {
                    #ident: ::core::default::Default::default()
                });
            }
            UploadKind::None => {
                assign.push(quote! { #ident: wire.#ident });
            }
        }
    }

    let construct = if let Some(submit_name) = submit_name {
        quote! { #submit_name { #(#assign),* } }
    } else {
        quote! { Self { #(#assign),* } }
    };

    Ok(quote! {
        #[derive(::serde::Deserialize)]
        struct __Wire {
            #(#wire_fields),*
        }
        let wire: __Wire = parts.text.deserialize()?;
        let _ = (#(&wire.#wire_idents,)*);
        Ok(#construct)
    })
}

// --- enum -------------------------------------------------------------------

fn expand_enum(input: &DeriveInput, args: &HtmlFormArgs) -> Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let vis = &input.vis;
    let Data::Enum(data) = &input.data else {
        unreachable!()
    };
    let tag = args.tag.clone().unwrap_or_else(|| "Kind".into());
    let model = {
        let mut c = tag.chars();
        match c.next() {
            Some(f) => format!("{}{}", f.to_lowercase(), c.as_str()),
            None => "kind".into(),
        }
    };
    let submit_name = format_ident!("{name}Submit");

    let mut def_variants = Vec::new();
    let mut submit_variants = Vec::new();
    let mut variant_specs = Vec::new();
    let mut assemble_arms = Vec::new();
    let mut file_ones = Vec::new();
    let mut file_manys = Vec::new();
    let mut first_variant_ident = None;

    for variant in &data.variants {
        let v_ident = &variant.ident;
        if first_variant_ident.is_none() {
            first_variant_ident = Some(v_ident.clone());
        }
        let v_form = parse_variant_form_attrs(variant)?;
        let v_label = v_form.label.clone().unwrap_or_else(|| v_ident.to_string());
        let v_value = v_ident.to_string();

        match &variant.fields {
            Fields::Unit => {
                def_variants.push(quote! { #v_ident });
                submit_variants.push(quote! { #v_ident });
                variant_specs.push(quote! {
                    ::lariv_rs::html_form::KindVariantSpec {
                        value: #v_value,
                        label: #v_label,
                        fields: &[],
                    }
                });
                assemble_arms.push(quote! {
                    #v_value => Ok(#submit_name::#v_ident)
                });
            }
            Fields::Named(named) => {
                let prepared: Vec<PreparedField> = named
                    .named
                    .iter()
                    .map(prepare_field)
                    .collect::<Result<_>>()?;
                let mut def_fields = Vec::new();
                let mut submit_fields = Vec::new();
                let mut specs = Vec::new();
                let mut assign = Vec::new();

                for f in &prepared {
                    let ident = &f.ident;
                    let ty = &f.ty;
                    let field_vis = &f.vis;
                    let serde_attrs = serde_attrs_for_field(f);
                    def_fields.push(quote! {
                        #[serde(#serde_attrs)]
                        #field_vis #ident: #ty
                    });
                    specs.push(field_spec_tokens(f));
                    let submit_ty = submit_ty_for_field(f);
                    submit_fields.push(quote! { #field_vis #ident: #submit_ty });
                    let html = &f.html_name;
                    match f.upload_kind {
                        UploadKind::One => {
                            file_ones.push(html.clone());
                            assign.push(quote! {
                                #ident: parts.files.remove(#html).ok_or_else(|| {
                                    ::lariv_rs::html_form::FormError::Validation(
                                        ::std::format!("{} is required", #html),
                                    )
                                })?
                            });
                        }
                        UploadKind::Optional => {
                            file_ones.push(html.clone());
                            assign.push(quote! { #ident: parts.files.remove(#html) });
                        }
                        UploadKind::Many => {
                            file_manys.push(html.clone());
                            assign.push(quote! {
                                #ident: parts.file_lists.remove(#html).unwrap_or_default()
                            });
                        }
                        UploadKind::None => {
                            // Text fields on variants: pull from text map with defaults.
                            if is_option(ty) {
                                assign.push(quote! {
                                    #ident: parts.text.get_first(#html).and_then(|s| {
                                        let s = s.trim();
                                        if s.is_empty() { None } else { Some(s.to_string()) }
                                    }).and_then(|s| s.parse().ok())
                                });
                            } else if is_integer(ty) {
                                assign.push(quote! {
                                    #ident: parts.text.get_first(#html)
                                        .map(|s| s.trim())
                                        .filter(|s| !s.is_empty())
                                        .and_then(|s| s.parse().ok())
                                        .unwrap_or(0)
                                });
                            } else {
                                // String etc.
                                assign.push(quote! {
                                    #ident: parts.text.get_first(#html)
                                        .map(|s| s.to_string())
                                        .unwrap_or_default()
                                });
                            }
                        }
                    }
                }

                def_variants.push(quote! {
                    #v_ident { #(#def_fields),* }
                });
                submit_variants.push(quote! {
                    #v_ident { #(#submit_fields),* }
                });
                variant_specs.push(quote! {
                    ::lariv_rs::html_form::KindVariantSpec {
                        value: #v_value,
                        label: #v_label,
                        fields: &[#(#specs),*],
                    }
                });
                assemble_arms.push(quote! {
                    #v_value => Ok(#submit_name::#v_ident { #(#assign),* })
                });
            }
            Fields::Unnamed(_) => {
                return Err(Error::new(
                    variant.span(),
                    "html_form enums do not support tuple variants",
                ));
            }
        }
    }

    let first = first_variant_ident
        .ok_or_else(|| Error::new(input.span(), "html_form enum needs at least one variant"))?;
    let first_variant = &data.variants[0];
    let default_expr = match &first_variant.fields {
        Fields::Unit => quote! { Self::#first },
        Fields::Named(named) => {
            let idents: Vec<_> = named
                .named
                .iter()
                .filter_map(|f| f.ident.as_ref())
                .collect();
            quote! {
                Self::#first {
                    #(#idents: ::core::default::Default::default()),*
                }
            }
        }
        Fields::Unnamed(_) => {
            return Err(Error::new(
                first_variant.span(),
                "html_form enums do not support tuple variants",
            ));
        }
    };

    let mut derives = vec![quote! { ::serde::Deserialize }];
    if !args.no_debug {
        derives.insert(0, quote! { ::core::fmt::Debug });
    }

    let enum_attrs: Vec<_> = input
        .attrs
        .iter()
        .filter(|a| !a.path().is_ident("derive"))
        .collect();

    let default_impl = if args.default {
        quote! {
            impl ::core::default::Default for #name {
                fn default() -> Self {
                    #default_expr
                }
            }
        }
    } else {
        quote! {}
    };

    let file_ones_lit = &file_ones;
    let file_manys_lit = &file_manys;

    let mut kind_fields: Vec<PreparedField> = Vec::new();
    for variant in &data.variants {
        if let Fields::Named(named) = &variant.fields {
            kind_fields.extend(
                named
                    .named
                    .iter()
                    .map(prepare_field)
                    .collect::<Result<Vec<_>>>()?,
            );
        }
    }
    let field_enum_name = format_ident!("{name}Field");
    let (field_enum, field_impl) = emit_field_key_enum(&field_enum_name, &kind_fields);

    Ok(quote! {
        #(#enum_attrs)*
        #[derive(#(#derives),*)]
        #[serde(tag = #tag)]
        #vis enum #name {
            #(#def_variants),*
        }

        #default_impl

        #field_enum
        #field_impl

        #[derive(Debug)]
        #vis enum #submit_name {
            #(#submit_variants),*
        }

        impl ::lariv_rs::html_form::HtmlForm for #name {
            type Field = #field_enum_name;
            type Flag = ::lariv_rs::html_form::NoFormFlags;
            type Submit = #submit_name;

            fn field_specs() -> &'static [::lariv_rs::html_form::FieldSpec] {
                &[]
            }

            fn file_field_names() -> &'static [&'static str] {
                &[#(#file_ones_lit),*]
            }

            fn multi_file_field_names() -> &'static [&'static str] {
                &[#(#file_manys_lit),*]
            }

            fn assemble_submit(
                mut parts: ::lariv_rs::html_form::MultipartParts,
            ) -> ::core::result::Result<Self::Submit, ::lariv_rs::html_form::FormError> {
                let value = parts
                    .text
                    .get_first(#tag)
                    .unwrap_or("")
                    .to_string();
                match value.as_str() {
                    #(#assemble_arms,)*
                    other => Err(::lariv_rs::html_form::FormError::Validation(
                        ::std::format!("unknown {}: {other}", #tag),
                    )),
                }
            }

            fn render_inputs(ctx: &::lariv_rs::html_form::FormCtx<'_>) -> ::maud::Markup {
                let field = ::lariv_rs::html_form::FieldRender {
                    name: #tag,
                    label: "",
                    value: ctx.value_of(#tag),
                    required: false,
                    spec: &::lariv_rs::html_form::FieldSpec {
                        name: #tag,
                        label: "",
                        required: false,
                        row: None,
                        when: None,
                        required_unless: None,
                        model: None,
                        show: None,
                        url: None,
                        swap_key: None,
                        display_key: None,
                        error_key: None,
                        choices_key: None,
                        placeholder: None,
                        hint: None,
                        rows: None,
                        multiple: false,
                        accept: None,
                        render: |_, _| ::maud::Markup::default(),
                    },
                };
                ::lariv_rs::html_form::render_kind::<Self>(ctx, &field)
            }
        }

        impl ::lariv_rs::html_form::HtmlKind for #name {
            fn kind_tag() -> &'static str { #tag }
            fn kind_model() -> &'static str { #model }
            fn variants() -> &'static [::lariv_rs::html_form::KindVariantSpec] {
                &[#(#variant_specs),*]
            }
        }
    })
}

// --- attrs / type helpers ---------------------------------------------------

fn opts_str(v: Option<&str>) -> proc_macro2::TokenStream {
    match v {
        Some(s) => quote! { ::core::option::Option::Some(#s) },
        None => quote! { ::core::option::Option::None },
    }
}

fn to_pascal_html_name(snake: &str) -> String {
    snake
        .split('_')
        .filter(|p| !p.is_empty())
        .map(|part| {
            if part.eq_ignore_ascii_case("id") {
                "ID".into()
            } else if part.eq_ignore_ascii_case("url") {
                "URL".into()
            } else {
                let mut c = part.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            }
        })
        .collect()
}

fn humanize_label(name: &str) -> String {
    name.replace('_', " ")
}

fn to_rust_variant_name(snake: &str) -> String {
    snake
        .split('_')
        .filter(|p| !p.is_empty())
        .map(|part| {
            let mut c = part.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

fn emit_field_key_enum(
    enum_name: &Ident,
    fields: &[PreparedField],
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let mut variants = Vec::new();
    let mut html_arms = Vec::new();
    let mut display_arms = Vec::new();
    let mut choices_arms = Vec::new();

    for f in fields {
        if f.is_kind {
            continue;
        }
        let variant = format_ident!("{}", to_rust_variant_name(&f.ident.to_string()));
        let html_name = &f.html_name;
        let display_key = f.form.display.as_deref().unwrap_or(html_name);
        let choices_key = f.form.choices.as_deref().unwrap_or(html_name);
        variants.push(variant.clone());
        html_arms.push(quote! { Self::#variant => #html_name, });
        display_arms.push(quote! { Self::#variant => #display_key, });
        choices_arms.push(quote! { Self::#variant => #choices_key, });
    }

    let enum_tokens = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[allow(non_camel_case_types)]
        pub enum #enum_name {
            #(#variants,)*
        }
    };

    let impl_tokens = quote! {
        impl ::lariv_rs::html_form::FormFieldKey for #enum_name {
            fn html_name(self) -> &'static str {
                match self {
                    #(#html_arms)*
                }
            }

            fn display_key(self) -> &'static str {
                match self {
                    #(#display_arms)*
                }
            }

            fn choices_key(self) -> &'static str {
                match self {
                    #(#choices_arms)*
                }
            }
        }
    };

    (enum_tokens, impl_tokens)
}

fn emit_flag_key_enum(
    enum_name: &Ident,
    fields: &[PreparedField],
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let mut flags: Vec<String> = fields
        .iter()
        .flat_map(|f| {
            f.form
                .when
                .iter()
                .chain(f.form.required_unless.iter())
                .cloned()
        })
        .collect();
    flags.sort();
    flags.dedup();

    if flags.is_empty() {
        return (
            quote! {
                #[derive(Debug, Clone, Copy)]
                pub enum #enum_name {}
            },
            quote! {
                impl ::lariv_rs::html_form::FormFlagKey for #enum_name {
                    fn as_str(self) -> &'static str {
                        match self {}
                    }
                }
            },
        );
    }

    let mut variants = Vec::new();
    let mut arms = Vec::new();
    for flag in flags {
        let variant = format_ident!("{}", to_rust_variant_name(&flag));
        variants.push(variant.clone());
        arms.push(quote! { Self::#variant => #flag, });
    }

    let enum_tokens = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum #enum_name {
            #(#variants,)*
        }
    };

    let impl_tokens = quote! {
        impl ::lariv_rs::html_form::FormFlagKey for #enum_name {
            fn as_str(self) -> &'static str {
                match self {
                    #(#arms)*
                }
            }
        }
    };

    (enum_tokens, impl_tokens)
}

fn widget_is_section(widget: &syn::Path) -> bool {
    widget.segments.last().is_some_and(|s| s.ident == "Section")
}

fn widget_is_kind(widget: &syn::Path) -> bool {
    widget.segments.last().is_some_and(|s| s.ident == "Kind")
}

fn classify_upload(ty: &Type) -> UploadKind {
    if path_last_ident(ty).as_deref() == Some("Upload") {
        return UploadKind::One;
    }
    if let Type::Path(p) = ty
        && let Some(seg) = p.path.segments.last()
    {
        if seg.ident == "Option"
            && let PathArguments::AngleBracketed(args) = &seg.arguments
            && let Some(GenericArgument::Type(inner)) = args.args.first()
            && path_last_ident(inner).as_deref() == Some("Upload")
        {
            return UploadKind::Optional;
        }
        if seg.ident == "Vec"
            && let PathArguments::AngleBracketed(args) = &seg.arguments
            && let Some(GenericArgument::Type(inner)) = args.args.first()
            && path_last_ident(inner).as_deref() == Some("Upload")
        {
            return UploadKind::Many;
        }
    }
    UploadKind::None
}

fn is_option(ty: &Type) -> bool {
    path_last_ident(ty).is_some_and(|id| id == "Option")
}

fn is_integer(ty: &Type) -> bool {
    matches!(
        path_last_ident(ty).as_deref(),
        Some("i64" | "u64" | "i32" | "u32" | "isize" | "usize")
    )
}

fn is_option_integer(ty: &Type) -> bool {
    let Type::Path(p) = ty else {
        return false;
    };
    let Some(seg) = p.path.segments.last() else {
        return false;
    };
    if seg.ident != "Option" {
        return false;
    }
    let PathArguments::AngleBracketed(args) = &seg.arguments else {
        return false;
    };
    matches!(args.args.first(), Some(GenericArgument::Type(inner)) if is_integer(inner))
}

fn is_vec_integer(ty: &Type) -> bool {
    let Type::Path(p) = ty else {
        return false;
    };
    let Some(seg) = p.path.segments.last() else {
        return false;
    };
    if seg.ident != "Vec" {
        return false;
    }
    let PathArguments::AngleBracketed(args) = &seg.arguments else {
        return false;
    };
    matches!(args.args.first(), Some(GenericArgument::Type(inner)) if is_integer(inner))
}

fn is_vec_string(ty: &Type) -> bool {
    let Type::Path(p) = ty else {
        return false;
    };
    let Some(seg) = p.path.segments.last() else {
        return false;
    };
    if seg.ident != "Vec" {
        return false;
    }
    let PathArguments::AngleBracketed(args) = &seg.arguments else {
        return false;
    };
    matches!(
        args.args.first(),
        Some(GenericArgument::Type(inner)) if path_last_ident(inner).as_deref() == Some("String")
    )
}

fn is_bool(ty: &Type) -> bool {
    matches!(path_last_ident(ty).as_deref(), Some("bool"))
}

fn matches_defaultable(ty: &Type) -> bool {
    matches!(path_last_ident(ty).as_deref(), Some("String" | "Vec"))
}

fn path_last_ident(ty: &Type) -> Option<String> {
    let Type::Path(p) = ty else {
        return None;
    };
    p.path.segments.last().map(|s| s.ident.to_string())
}

enum HintExpr {
    Lit(String),
    Path(syn::Path),
}

#[derive(Default)]
struct FormAttrs {
    label: Option<String>,
    name: Option<String>,
    widget: Option<syn::Path>,
    required: bool,
    multiple: bool,
    skip_deser: bool,
    url: Option<String>,
    swap_key: Option<String>,
    display: Option<String>,
    error: Option<String>,
    choices: Option<String>,
    when: Option<String>,
    required_unless: Option<String>,
    model: Option<String>,
    show: Option<String>,
    placeholder: Option<String>,
    accept: Option<String>,
    row: Option<String>,
    rows: Option<u32>,
    hint: Option<HintExpr>,
    route: Option<syn::Path>,
}

fn parse_form_attrs(field: &syn::Field) -> Result<FormAttrs> {
    parse_form_attr_list(&field.attrs)
}

fn parse_variant_form_attrs(variant: &syn::Variant) -> Result<FormAttrs> {
    parse_form_attr_list(&variant.attrs)
}

fn parse_form_attr_list(attrs: &[syn::Attribute]) -> Result<FormAttrs> {
    let mut out = FormAttrs::default();
    for attr in attrs {
        if !attr.path().is_ident("form") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("required") {
                out.required = true;
                return Ok(());
            }
            if meta.path.is_ident("multiple") {
                out.multiple = true;
                return Ok(());
            }
            if meta.path.is_ident("skip_deser") {
                out.skip_deser = true;
                return Ok(());
            }
            if meta.path.is_ident("widget") {
                let value = meta.value()?;
                out.widget = Some(value.parse()?);
                return Ok(());
            }
            if meta.path.is_ident("label") {
                out.label = Some(parse_str(&meta)?);
                return Ok(());
            }
            if meta.path.is_ident("name") {
                out.name = Some(parse_str(&meta)?);
                return Ok(());
            }
            if meta.path.is_ident("url") {
                out.url = Some(parse_str(&meta)?);
                return Ok(());
            }
            if meta.path.is_ident("swap_key") {
                out.swap_key = Some(parse_str(&meta)?);
                return Ok(());
            }
            if meta.path.is_ident("display") {
                out.display = Some(parse_str(&meta)?);
                return Ok(());
            }
            if meta.path.is_ident("error") {
                out.error = Some(parse_str(&meta)?);
                return Ok(());
            }
            if meta.path.is_ident("choices") {
                out.choices = Some(parse_str(&meta)?);
                return Ok(());
            }
            if meta.path.is_ident("when") {
                out.when = Some(parse_str(&meta)?);
                return Ok(());
            }
            if meta.path.is_ident("required_unless") {
                out.required_unless = Some(parse_str(&meta)?);
                return Ok(());
            }
            if meta.path.is_ident("model") {
                out.model = Some(parse_str(&meta)?);
                return Ok(());
            }
            if meta.path.is_ident("show") {
                out.show = Some(parse_str(&meta)?);
                return Ok(());
            }
            if meta.path.is_ident("placeholder") {
                out.placeholder = Some(parse_str(&meta)?);
                return Ok(());
            }
            if meta.path.is_ident("accept") {
                out.accept = Some(parse_str(&meta)?);
                return Ok(());
            }
            if meta.path.is_ident("row") {
                out.row = Some(parse_str(&meta)?);
                return Ok(());
            }
            if meta.path.is_ident("rows") {
                let value = meta.value()?;
                let lit: syn::LitInt = value.parse()?;
                out.rows = Some(lit.base10_parse()?);
                return Ok(());
            }
            if meta.path.is_ident("hint") {
                let value = meta.value()?;
                let expr: syn::Expr = value.parse()?;
                out.hint = Some(match expr {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) => HintExpr::Lit(s.value()),
                    syn::Expr::Path(p) => HintExpr::Path(p.path),
                    other => {
                        return Err(Error::new_spanned(
                            other,
                            "hint expects a string literal or path to a &'static str",
                        ));
                    }
                });
                return Ok(());
            }
            if meta.path.is_ident("route") {
                let value = meta.value()?;
                out.route = Some(value.parse()?);
                return Ok(());
            }
            Err(meta.error("unsupported form attribute"))
        })?;
    }
    Ok(out)
}

fn parse_str(meta: &syn::meta::ParseNestedMeta<'_>) -> Result<String> {
    let value = meta.value()?;
    let lit: syn::LitStr = value.parse()?;
    Ok(lit.value())
}
