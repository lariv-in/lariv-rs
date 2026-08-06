//! Attribute-macro-backed HTML forms: [`HtmlForm`] + [`FormWidget`].
//!
//! Define a `*Form` struct with `#[html_form]` to get compile-time field specs,
//! generated `{Form}Field` / `{Form}Flag` enums, Maud rendering via [`FormWidget`],
//! and multipart parsing via [`HtmlForm::from_multipart`].
//!
//! For urlencoded POST handlers use [`HtmlFormBody`] instead of [`axum::Form`]
//! so many-to-many fields (`Vec<i64>`) with repeated HTML names deserialize correctly.
//!
//! # When to use
//!
//! Use for create/edit wizards, credentials forms, and query filters where the
//! same struct drives both HTML and submission parsing. Hand-built pages can use
//! [`crate::components::input`] directly instead.
//!
//! ```rust,ignore
//! #[derive(Default)]
//! #[html_form(action = "/users", enctype = "multipart/form-data")]
//! struct UserForm {
//!     #[widget(Text)]
//!     name: String,
//!     #[widget(Email)]
//!     email: String,
//! }
//!
//! // Handler: UserForm::from_multipart(multipart).await
//! // GET page: UserForm::render_inputs(&ctx)
//! ```

pub mod extract;
pub mod multipart;
pub mod upload;
pub mod urlencoded;
pub mod widgets;

use std::{borrow::Cow, collections::HashMap, fmt, marker::PhantomData, ops::Deref, str::FromStr};

use axum::extract::Multipart;
use maud::{Markup, html};
use serde::{Deserialize, Deserializer};

use crate::components::{ManyToManyItem, container_error, container_row};

pub use lariv_rs_macros::html_form;
pub use extract::HtmlFormBody;
pub use multipart::{MultipartParts, collect_multipart, deserialize_text_map};
pub use urlencoded::{deserialize_urlencoded, parse_urlencoded_form};
pub use upload::{Upload, UploadedFile};
pub use widgets::*;

/// Errors from multipart collection / form assembly.
#[derive(Debug, thiserror::Error)]
pub enum FormError {
    #[error("multipart: {0}")]
    Multipart(String),
    #[error("upload spool: {0}")]
    Spool(String),
    #[error("deserialize: {0}")]
    Deserialize(String),
    #[error("{0}")]
    Validation(String),
}

/// Preprocess integer/decimal form input: trim and strip thousands separators (`,`).
pub fn preprocess_numeric_form_value(s: &str) -> std::borrow::Cow<'_, str> {
    let s = s.trim();
    if s.contains(',') {
        std::borrow::Cow::Owned(s.chars().filter(|&c| c != ',').collect())
    } else {
        std::borrow::Cow::Borrowed(s)
    }
}

/// HTML forms send empty inputs as `""`; serde's `Option<i64>` rejects that.
pub fn empty_str_as_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    let s = Option::<String>::deserialize(deserializer)?;
    match s.as_deref().map(preprocess_numeric_form_value) {
        None => Ok(None),
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => T::from_str(s.as_ref())
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}

/// Empty string → `0` for non-optional integer form fields (FK pickers).
pub fn empty_str_as_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let s = preprocess_numeric_form_value(&s);
    if s.is_empty() {
        Ok(0)
    } else {
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// HTML forms send one value (`Tags=1`) or many (`Tags=1&Tags=2`) for the same key.
pub fn form_vec_string<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }

    match Option::<OneOrMany>::deserialize(deserializer)? {
        None => Ok(vec![]),
        Some(OneOrMany::One(s)) => {
            let s = s.trim();
            if s.is_empty() {
                Ok(vec![])
            } else {
                Ok(vec![s.to_string()])
            }
        }
        Some(OneOrMany::Many(items)) => Ok(items
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()),
    }
}

/// HTML forms send one value (`Tags=1`) or many (`Tags=1&Tags=2`) for the same key.
pub fn form_vec_i64<'de, D>(deserializer: D) -> Result<Vec<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }

    match Option::<OneOrMany>::deserialize(deserializer)? {
        None => Ok(vec![]),
        Some(OneOrMany::One(s)) => parse_form_vec_i64(&s).map_err(serde::de::Error::custom),
        Some(OneOrMany::Many(items)) => {
            let mut out = Vec::new();
            for s in items {
                out.extend(parse_form_vec_i64(&s).map_err(serde::de::Error::custom)?);
            }
            Ok(out)
        }
    }
}

fn parse_form_vec_i64(s: &str) -> Result<Vec<i64>, String> {
    let s = preprocess_numeric_form_value(s);
    if s.is_empty() {
        return Ok(vec![]);
    }
    s.parse::<i64>()
        .map(|n| vec![n])
        .map_err(|e| e.to_string())
}

/// Compile-time HTML input name for a form field (generated per `#[html_form]` struct).
pub trait FormFieldKey: Copy {
    fn html_name(self) -> &'static str;
    fn display_key(self) -> &'static str {
        self.html_name()
    }
    fn choices_key(self) -> &'static str {
        self.html_name()
    }
    /// Picker `target_input` query param — same as [`Self::html_name`].
    fn target_input(self) -> &'static str {
        self.html_name()
    }
}

/// Server-side visibility / conditional flag (generated from `when` / `required_unless`).
pub trait FormFlagKey: Copy {
    fn as_str(self) -> &'static str;
}

/// Placeholder for tagged enum forms without flat field lists.
#[derive(Debug, Clone, Copy)]
pub enum NoFormFields {}

impl FormFieldKey for NoFormFields {
    fn html_name(self) -> &'static str {
        match self {}
    }
}

/// Placeholder for forms without conditional flags.
#[derive(Debug, Clone, Copy)]
pub enum NoFormFlags {}

impl FormFlagKey for NoFormFlags {
    fn as_str(self) -> &'static str {
        match self {}
    }
}

/// Type-safe builder for [`FormCtx`] — use [`FormCtx::form`] and field keys from the
/// generated `{Form}Field` / `{Form}Flag` enums.
pub struct FormCtxBuilder<'a, F: HtmlForm> {
    ctx: FormCtx<'a>,
    _form: PhantomData<F>,
}

impl<'a, F: HtmlForm> FormCtxBuilder<'a, F> {
    pub fn value(mut self, field: impl FormFieldKey, value: impl Into<Cow<'a, str>>) -> Self {
        self.ctx = self.ctx.set_value(field.html_name(), value);
        self
    }

    pub fn checked(mut self, field: impl FormFieldKey, checked: bool) -> Self {
        self.ctx = self.ctx.set_checked(field.html_name(), checked);
        self
    }

    pub fn error(mut self, field: impl FormFieldKey, error: Option<&'a str>) -> Self {
        self.ctx = self.ctx.set_error(field.display_key(), error);
        self
    }

    pub fn flag(mut self, flag: F::Flag, on: bool) -> Self {
        self.ctx = self.ctx.set_flag(flag.as_str(), on);
        self
    }

    pub fn choices(mut self, field: impl FormFieldKey, choices: &'a [(String, String)]) -> Self {
        self.ctx = self.ctx.set_choices(field.choices_key(), choices);
        self
    }

    pub fn m2m(mut self, field: impl FormFieldKey, items: &'a [ManyToManyItem]) -> Self {
        self.ctx = self.ctx.set_m2m(field.html_name(), items);
        self
    }

    pub fn display(mut self, field: impl FormFieldKey, display: &'a str) -> Self {
        self.ctx = self.ctx.set_display(field.display_key(), display);
        self
    }

    pub fn url(mut self, field: impl FormFieldKey, url: &'a str) -> Self {
        self.ctx = self.ctx.set_url(field.html_name(), url);
        self
    }

    pub fn label(mut self, field: impl FormFieldKey, label: &'a str) -> Self {
        self.ctx = self.ctx.set_label(field.html_name(), label);
        self
    }

    pub fn x_data(mut self, data: &'a str) -> Self {
        self.ctx = self.ctx.set_x_data(data);
        self
    }

    pub fn lock_kind(mut self, locked: bool) -> Self {
        self.ctx = self.ctx.set_lock_kind(locked);
        self
    }

    /// Set the tagged enum discriminant for forms with a [`Kind`] widget.
    pub fn kind<K: HtmlKind>(mut self, value: &'a str) -> Self {
        self.ctx = self.ctx.set_value(K::kind_tag(), value);
        self
    }

    pub fn into_ctx(self) -> FormCtx<'a> {
        self.ctx
    }
}

impl<'a, F: HtmlForm> Deref for FormCtxBuilder<'a, F> {
    type Target = FormCtx<'a>;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl<'a, F: HtmlForm> From<FormCtxBuilder<'a, F>> for FormCtx<'a> {
    fn from(builder: FormCtxBuilder<'a, F>) -> Self {
        builder.ctx
    }
}

/// One widget implementation — stock ([`widgets`]) and app widgets use this trait.
///
/// Implement for custom field types; reference the type in `#[widget(MyWidget)]`.
pub trait FormWidget {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup;
}

/// Per-field view passed to [`FormWidget::render`].
pub struct FieldRender<'a> {
    pub name: &'a str,
    pub label: &'a str,
    pub value: &'a str,
    pub required: bool,
    pub spec: &'a FieldSpec,
}

/// Compile-time description of one form field (generated by `#[html_form]`).
pub struct FieldSpec {
    pub name: &'static str,
    pub label: &'static str,
    pub required: bool,
    pub row: Option<&'static str>,
    /// Server-side visibility flag (`FormCtx::flag`).
    pub when: Option<&'static str>,
    pub required_unless: Option<&'static str>,
    /// Alpine.js `x-model` binding (typically on checkboxes).
    pub model: Option<&'static str>,
    /// Alpine.js expression for client-side `x-show` (requires [`FormCtx::x_data`]).
    pub show: Option<&'static str>,
    pub url: Option<&'static str>,
    pub swap_key: Option<&'static str>,
    pub display_key: Option<&'static str>,
    pub error_key: Option<&'static str>,
    pub choices_key: Option<&'static str>,
    pub placeholder: Option<&'static str>,
    pub rows: Option<u32>,
    pub multiple: bool,
    pub accept: Option<&'static str>,
    pub render: fn(&FormCtx<'_>, &FieldRender<'_>) -> Markup,
}

/// One variant of a tagged [`HtmlKind`] enum form.
pub struct KindVariantSpec {
    pub value: &'static str,
    pub label: &'static str,
    pub fields: &'static [FieldSpec],
}

/// Tagged enum forms: discriminant radios + per-variant fields.
///
/// Use when one form shape depends on a selected kind (e.g. payment method).
pub trait HtmlKind: HtmlForm {
    fn kind_tag() -> &'static str;
    /// Alpine / JS property for `x-model` (camelCase).
    fn kind_model() -> &'static str;
    fn variants() -> &'static [KindVariantSpec];
}

/// Request `*Form` types that expose field specs for rendering and multipart submit.
///
/// Generated by `#[html_form]`; call [`Self::render_inputs`] on GET and
/// [`Self::from_multipart`] on POST.
pub trait HtmlForm: Sized {
    /// Generated `{Self}Field` enum — use with [`FormCtx::form`].
    type Field: FormFieldKey;
    /// Generated `{Self}Flag` enum for `when` / `required_unless` attrs.
    type Flag: FormFlagKey;

    /// Parsed submission type (`Upload` → [`UploadedFile`]).
    type Submit;

    fn field_specs() -> &'static [FieldSpec];

    fn file_field_names() -> &'static [&'static str] {
        &[]
    }

    fn multi_file_field_names() -> &'static [&'static str] {
        &[]
    }

    fn assemble_submit(parts: MultipartParts) -> Result<Self::Submit, FormError>;

    fn render_inputs(ctx: &FormCtx<'_>) -> Markup {
        render_field_specs(Self::field_specs(), ctx)
    }

    fn from_multipart(
        multipart: Multipart,
    ) -> impl std::future::Future<Output = Result<Self::Submit, FormError>> + Send {
        async move {
            let parts = collect_multipart(
                multipart,
                Self::file_field_names(),
                Self::multi_file_field_names(),
            )
            .await?;
            Self::assemble_submit(parts)
        }
    }

    /// Deserialize `application/x-www-form-urlencoded` bodies (supports duplicate keys).
    fn from_urlencoded(body: &[u8]) -> Result<Self, FormError>
    where
        Self: serde::de::DeserializeOwned,
    {
        urlencoded::deserialize_urlencoded(body)
    }
}

/// Runtime values, errors, and flags for rendering a form.
///
/// Construct only via [`FormCtx::form`] and its [`FormCtxBuilder`].
#[derive(Default)]
pub struct FormCtx<'a> {
    values: HashMap<&'a str, Cow<'a, str>>,
    checked: HashMap<&'a str, bool>,
    errors: HashMap<&'a str, &'a str>,
    flags: HashMap<&'a str, bool>,
    choices: HashMap<&'a str, &'a [(String, String)]>,
    m2m: HashMap<&'a str, &'a [ManyToManyItem]>,
    displays: HashMap<&'a str, &'a str>,
    urls: HashMap<&'a str, &'a str>,
    labels: HashMap<&'a str, &'a str>,
    /// Alpine.js `x-data` object literal wrapping the rendered inputs.
    x_data: Option<&'a str>,
    kind_locked: bool,
}

impl FormCtx<'_> {
    /// Start a type-safe builder keyed to `F`'s generated field / flag enums.
    pub fn form<'a, F: HtmlForm>() -> FormCtxBuilder<'a, F> {
        FormCtxBuilder {
            ctx: FormCtx::default(),
            _form: PhantomData,
        }
    }
}

impl<'a> FormCtx<'a> {
    pub(crate) fn set_value(mut self, name: &'a str, value: impl Into<Cow<'a, str>>) -> Self {
        self.values.insert(name, value.into());
        self
    }

    pub(crate) fn set_checked(mut self, name: &'a str, checked: bool) -> Self {
        self.checked.insert(name, checked);
        self
    }

    pub(crate) fn set_error(mut self, key: &'a str, error: Option<&'a str>) -> Self {
        if let Some(msg) = error.filter(|m| !m.is_empty()) {
            self.errors.insert(key, msg);
        }
        self
    }

    pub(crate) fn set_flag(mut self, key: &'a str, on: bool) -> Self {
        self.flags.insert(key, on);
        self
    }

    pub(crate) fn set_choices(mut self, key: &'a str, choices: &'a [(String, String)]) -> Self {
        self.choices.insert(key, choices);
        self
    }

    pub(crate) fn set_m2m(mut self, name: &'a str, items: &'a [ManyToManyItem]) -> Self {
        self.m2m.insert(name, items);
        self
    }

    pub(crate) fn set_display(mut self, key: &'a str, display: &'a str) -> Self {
        self.displays.insert(key, display);
        self
    }

    pub(crate) fn set_url(mut self, name: &'a str, url: &'a str) -> Self {
        self.urls.insert(name, url);
        self
    }

    pub(crate) fn set_label(mut self, name: &'a str, label: &'a str) -> Self {
        self.labels.insert(name, label);
        self
    }

    pub(crate) fn set_x_data(mut self, data: &'a str) -> Self {
        self.x_data = Some(data);
        self
    }

    pub(crate) fn set_lock_kind(mut self, locked: bool) -> Self {
        self.kind_locked = locked;
        self
    }

    pub fn kind_locked(&self) -> bool {
        self.kind_locked
    }

    pub fn flag_on(&self, key: &str) -> bool {
        self.flags.get(key).copied().unwrap_or(false)
    }

    pub fn value_of(&self, name: &str) -> &str {
        self.values.get(name).map(|c| c.as_ref()).unwrap_or("")
    }

    pub fn checked_of(&self, name: &str) -> bool {
        self.checked.get(name).copied().unwrap_or(false)
    }

    pub fn label_of(&self, spec: &FieldSpec) -> &str {
        self.labels.get(spec.name).copied().unwrap_or(spec.label)
    }

    pub fn url_of(&self, spec: &FieldSpec) -> &str {
        self.urls
            .get(spec.name)
            .copied()
            .or(spec.url)
            .unwrap_or("")
    }

    pub fn display_of(&self, key: &str) -> &str {
        self.displays
            .get(key)
            .copied()
            .or_else(|| self.values.get(key).map(|c| c.as_ref()))
            .unwrap_or("")
    }

    pub fn choices_of(&self, key: &str) -> &[(String, String)] {
        self.choices.get(key).copied().unwrap_or(&[])
    }

    pub fn m2m_of(&self, name: &str) -> &[ManyToManyItem] {
        self.m2m.get(name).copied().unwrap_or(&[])
    }

    pub fn error_of(&self, spec: &FieldSpec) -> Option<&str> {
        let key = spec.error_key.unwrap_or(spec.name);
        self.errors.get(key).copied()
    }
}

/// Render a tagged [`HtmlKind`] enum (radios + variant fields).
///
/// Called by the `Kind` widget; rarely invoked directly.
pub fn render_kind<K: HtmlKind>(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
    let tag = K::kind_tag();
    let model = K::kind_model();
    let selected = {
        let v = ctx.value_of(field.name);
        if v.is_empty() {
            ctx.value_of(tag)
        } else {
            v
        }
    };
    let selected = if selected.is_empty() {
        K::variants()
            .first()
            .map(|v| v.value)
            .unwrap_or("")
    } else {
        selected
    };

    if ctx.kind_locked() {
        let mut out = Markup::default();
        for variant in K::variants() {
            if variant.value == selected {
                out = html! { (out) (render_field_specs(variant.fields, ctx)) };
            }
        }
        return out;
    }

    let options: Vec<crate::components::InputRadioOption<'_>> = K::variants()
        .iter()
        .map(|v| crate::components::InputRadioOption {
            value: v.value,
            label: v.label,
            checked: v.value == selected,
        })
        .collect();
    let radios = crate::components::input_radio_group(crate::components::InputRadioGroup {
        label: if field.label.is_empty() {
            ""
        } else {
            field.label
        },
        name: tag,
        options: &options,
        attrs: crate::components::HtmlAttrs::new().set("x-model", model),
        ..Default::default()
    });

    let mut body = radios;
    for variant in K::variants() {
        let expr = format!("{model} === '{}'", variant.value);
        let fields = render_field_specs(variant.fields, ctx);
        body = html! {
            (body)
            div x-show=(expr) {
                (fields)
            }
        };
    }

    let x_data = format!("{{ {model}: '{selected}' }}");
    html! {
        div x-data=(x_data) {
            (body)
        }
    }
}

/// Render field specs with optional row grouping and Alpine wrapper.
pub fn render_field_specs(specs: &[FieldSpec], ctx: &FormCtx<'_>) -> Markup {
    let visible: Vec<&FieldSpec> = specs.iter().filter(|s| is_visible(s, ctx)).collect();
    let mut out = Markup::default();
    let mut i = 0;
    while i < visible.len() {
        let spec = visible[i];
        if let Some(row_id) = spec.row {
            let start = i;
            i += 1;
            while i < visible.len() && visible[i].row == Some(row_id) {
                i += 1;
            }
            let group = &visible[start..i];
            let n = group.len();
            let class = format!("grid grid-cols-1 gap-1 @md:grid-cols-{n}");
            let cells = html! {
                @for s in group.iter() {
                    (render_one(s, ctx))
                }
            };
            out = html! { (out) (container_row(&class, cells)) };
        } else {
            out = html! { (out) (render_one(spec, ctx)) };
            i += 1;
        }
    }
    match ctx.x_data {
        Some(data) => html! {
            div x-data=(data) {
                (out)
            }
        },
        None => out,
    }
}

fn is_visible(spec: &FieldSpec, ctx: &FormCtx<'_>) -> bool {
    match spec.when {
        Some(flag) => ctx.flag_on(flag),
        None => true,
    }
}

/// Whether a field is required given `required_unless` flags.
pub fn field_required(spec: &FieldSpec, ctx: &FormCtx<'_>) -> bool {
    if let Some(flag) = spec.required_unless {
        return !ctx.flag_on(flag);
    }
    spec.required
}

fn render_one(spec: &FieldSpec, ctx: &FormCtx<'_>) -> Markup {
    let required = field_required(spec, ctx);
    let field = FieldRender {
        name: spec.name,
        label: ctx.label_of(spec),
        value: ctx.value_of(spec.name),
        required,
        spec,
    };
    let markup = (spec.render)(ctx, &field);
    let wrapped = container_error(ctx.error_of(spec), markup);
    match spec.show {
        Some(expr) => html! {
            div x-show=(expr) {
                (wrapped)
            }
        },
        None => wrapped,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{deserialize_text_map, form_vec_i64, form_vec_string, FormCtx};
    use serde::Deserialize;

    #[test]
    fn form_vec_string_accepts_single_urlencoded_value() {
        let form: ModelsForm =
            serde_json::from_value(serde_json::json!({"models": "tallies"})).expect("single model");
        assert_eq!(form.models, vec!["tallies".to_string()]);
    }

    #[test]
    fn form_vec_string_accepts_multiple_urlencoded_values() {
        let form: ModelsForm = serde_json::from_value(serde_json::json!({
            "models": ["tallies", "tot_school_sessions"]
        }))
        .expect("multiple models");
        assert_eq!(
            form.models,
            vec!["tallies".to_string(), "tot_school_sessions".to_string()]
        );
    }

    #[derive(Debug, Deserialize)]
    struct ModelsForm {
        #[serde(default, rename = "models", deserialize_with = "form_vec_string")]
        models: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    struct TagsForm {
        #[serde(rename = "Tags", default, deserialize_with = "form_vec_i64")]
        tags: Vec<i64>,
    }

    #[test]
    fn preprocess_numeric_form_value_strips_commas() {
        assert_eq!(
            super::preprocess_numeric_form_value("1,234").as_ref(),
            "1234"
        );
        assert_eq!(
            super::preprocess_numeric_form_value(" 1,234.50 ").as_ref(),
            "1234.50"
        );
        assert_eq!(super::preprocess_numeric_form_value("42").as_ref(), "42");
    }

    #[test]
    fn empty_str_as_i64_strips_commas() {
        #[derive(Debug, Deserialize)]
        struct NumForm {
            #[serde(deserialize_with = "super::empty_str_as_i64")]
            n: i64,
        }
        let form: NumForm =
            serde_json::from_value(serde_json::json!({"n": "1,234"})).expect("comma int");
        assert_eq!(form.n, 1234);
    }

    #[test]
    fn form_vec_i64_accepts_single_urlencoded_value() {
        let form: TagsForm = serde_json::from_value(serde_json::json!({"Tags": "1"}))
            .expect("single tag");
        assert_eq!(form.tags, vec![1]);
    }

    #[test]
    fn form_vec_i64_accepts_multiple_urlencoded_values() {
        let form: TagsForm =
            serde_json::from_value(serde_json::json!({"Tags": ["1", "2"]})).expect("multiple tags");
        assert_eq!(form.tags, vec![1, 2]);
    }

    #[test]
    fn form_vec_i64_accepts_json_string_from_text_map() {
        let mut text = HashMap::new();
        text.insert("Tags".into(), vec!["1".into()]);
        let form: TagsForm = deserialize_text_map(&text).expect("json string");
        assert_eq!(form.tags, vec![1]);
    }

    #[test]
    fn display_of_falls_back_to_value_map() {
        let ctx = FormCtx::default().set_value("parent_display", "Cash");
        assert_eq!(ctx.display_of("parent_display"), "Cash");
    }

    #[test]
    fn display_of_prefers_display_map() {
        let ctx = FormCtx::default()
            .set_value("parent_display", "wrong")
            .set_display("parent_display", "Cash");
        assert_eq!(ctx.display_of("parent_display"), "Cash");
    }
}
