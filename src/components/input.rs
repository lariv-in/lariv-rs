//! Presentational form inputs for hand-built forms and [`crate::html_form`] widgets.
//!
//! Each builder emits a labeled control with DaisyUI classes. Pair with
//! [`crate::components::form::form`] or let `#[html_form]` pick widgets automatically.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::{HtmlAttrs, escape_attr};

struct LabeledInput<'a> {
    wrap_class: &'a str,
    label: &'a str,
    show_label: bool,
    input_type: &'a str,
    name: &'a str,
    value: &'a str,
    input_class: &'a str,
    required: bool,
    attrs: &'a HtmlAttrs,
}

fn labeled_input(opts: LabeledInput<'_>) -> Markup {
    let required_attr = if opts.required { " required" } else { "" };
    html! {
        div class=(opts.wrap_class) {
            label class="label text-sm font-bold flex flex-col items-start gap-1" {
                @if opts.show_label { (opts.label) }
                (PreEscaped(format!(
                    r#"<input type="{}" name="{}" value="{}" class="{}"{}{}>"#,
                    escape_attr(opts.input_type),
                    escape_attr(opts.name),
                    escape_attr(opts.value),
                    escape_attr(opts.input_class),
                    required_attr,
                    opts.attrs.as_string()
                )))
            }
        }
    }
}

/// Single-line text input (supports hidden mode for CSRF/extra fields).
pub struct InputText<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub value: &'a str,
    pub required: bool,
    pub hidden: bool,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputText<'_> {
    fn default() -> Self {
        Self {
            label: "",
            name: "",
            value: "",
            required: false,
            hidden: false,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a text input.
pub fn input_text(opts: InputText<'_>) -> Markup {
    let wrap = if opts.hidden {
        format!("my-1 hidden {}", opts.classes)
    } else {
        format!("my-1 {}", opts.classes)
    };
    let input_type = if opts.hidden { "hidden" } else { "text" };
    let input_class = format!("input input-bordered w-full {}", opts.classes);
    labeled_input(LabeledInput {
        wrap_class: &wrap,
        label: opts.label,
        show_label: !opts.hidden,
        input_type,
        name: opts.name,
        value: opts.value,
        input_class: &input_class,
        required: opts.required,
        attrs: &opts.attrs,
    })
}

/// HTML color picker (`#rrggbb`).
pub struct InputColor<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub value: &'a str,
    pub required: bool,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputColor<'_> {
    fn default() -> Self {
        Self {
            label: "Color",
            name: "color",
            value: "",
            required: false,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a color input.
pub fn input_color(opts: InputColor<'_>) -> Markup {
    let wrap = format!("my-1 {}", opts.classes);
    let value = if opts.value.is_empty() {
        "#6366f1"
    } else {
        opts.value
    };
    labeled_input(LabeledInput {
        wrap_class: &wrap,
        label: opts.label,
        show_label: true,
        input_type: "color",
        name: opts.name,
        value,
        input_class: "h-10 w-16 cursor-pointer p-1 rounded-md border border-base-300 bg-base-100",
        required: opts.required,
        attrs: &opts.attrs,
    })
}

/// Email input with browser validation.
pub struct InputEmail<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub value: &'a str,
    pub required: bool,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputEmail<'_> {
    fn default() -> Self {
        Self {
            label: "Email",
            name: "email",
            value: "",
            required: false,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render an email input.
pub fn input_email(opts: InputEmail<'_>) -> Markup {
    let wrap = format!("my-1 {}", opts.classes);
    let input_class = format!("input input-bordered w-full {}", opts.classes);
    labeled_input(LabeledInput {
        wrap_class: &wrap,
        label: opts.label,
        show_label: true,
        input_type: "email",
        name: opts.name,
        value: opts.value,
        input_class: &input_class,
        required: opts.required,
        attrs: &opts.attrs,
    })
}

/// Password input (value not repopulated on validation errors by default).
pub struct InputPassword<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub value: &'a str,
    pub required: bool,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputPassword<'_> {
    fn default() -> Self {
        Self {
            label: "Password",
            name: "password",
            value: "",
            required: false,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a password input.
pub fn input_password(opts: InputPassword<'_>) -> Markup {
    let wrap = format!("my-1 {}", opts.classes);
    let input_class = format!("input input-bordered w-full {}", opts.classes);
    labeled_input(LabeledInput {
        wrap_class: &wrap,
        label: opts.label,
        show_label: true,
        input_type: "password",
        name: opts.name,
        value: opts.value,
        input_class: &input_class,
        required: opts.required,
        attrs: &opts.attrs,
    })
}

/// Numeric input (`type="number"`).
pub struct InputNumber<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub value: &'a str,
    pub required: bool,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputNumber<'_> {
    fn default() -> Self {
        Self {
            label: "",
            name: "",
            value: "",
            required: false,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a number input.
pub fn input_number(opts: InputNumber<'_>) -> Markup {
    let wrap = format!("my-1 {}", opts.classes);
    let input_class = format!("input input-bordered w-full {}", opts.classes);
    labeled_input(LabeledInput {
        wrap_class: &wrap,
        label: opts.label,
        show_label: true,
        input_type: "number",
        name: opts.name,
        value: opts.value,
        input_class: &input_class,
        required: opts.required,
        attrs: &opts.attrs,
    })
}

/// Multi-line text area.
pub struct InputTextarea<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub value: &'a str,
    pub required: bool,
    pub rows: u32,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputTextarea<'_> {
    fn default() -> Self {
        Self {
            label: "",
            name: "",
            value: "",
            required: false,
            rows: 4,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a textarea.
pub fn input_textarea(opts: InputTextarea<'_>) -> Markup {
    let required_attr = if opts.required { " required" } else { "" };
    html! {
        div class=(format!("my-1 {}", opts.classes)) {
            label class="label text-sm font-bold flex flex-col items-start gap-1" {
                (opts.label)
                (PreEscaped(format!(
                    r#"<textarea name="{}" rows="{}" class="{}"{}{}>"#,
                    escape_attr(opts.name),
                    opts.rows,
                    escape_attr(&format!("textarea textarea-bordered w-full {}", opts.classes)),
                    required_attr,
                    opts.attrs.as_string()
                )))
                (opts.value)
                (PreEscaped("</textarea>"))
            }
        }
    }
}

/// Checkbox or hidden boolean (hidden emits `true`/`false` string).
pub struct InputCheckbox<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub checked: bool,
    pub hidden: bool,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputCheckbox<'_> {
    fn default() -> Self {
        Self {
            label: "",
            name: "",
            checked: false,
            hidden: false,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a checkbox input.
pub fn input_checkbox(opts: InputCheckbox<'_>) -> Markup {
    if opts.hidden {
        return html! {
            div class="hidden" {
                input type="hidden" name=(opts.name) value=(if opts.checked { "true" } else { "false" });
            }
        };
    }
    let checked_attr = if opts.checked { " checked" } else { "" };
    html! {
        div class=(opts.classes) {
            label class="label text-sm font-bold cursor-pointer justify-start gap-2 flex flex-row items-center" {
                (PreEscaped(format!(
                    r#"<input type="checkbox" name="{}" value="true" class="checkbox"{}{}>"#,
                    escape_attr(opts.name),
                    checked_attr,
                    opts.attrs.as_string()
                )))
                span class="label-text" { (opts.label) }
            }
        }
    }
}

/// One option in an [`InputRadioGroup`].
pub struct InputRadioOption<'a> {
    pub value: &'a str,
    pub label: &'a str,
    pub checked: bool,
}

/// Mutually exclusive radio button group.
pub struct InputRadioGroup<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub options: &'a [InputRadioOption<'a>],
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputRadioGroup<'_> {
    fn default() -> Self {
        Self {
            label: "",
            name: "",
            options: &[],
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a radio group (supports Alpine `x-model` via attrs).
pub fn input_radio_group(opts: InputRadioGroup<'_>) -> Markup {
    html! {
        div class=(format!("my-1 {}", opts.classes)) {
            @if !opts.label.is_empty() {
                div class="label text-sm font-bold" { (opts.label) }
            }
            div class="flex flex-col gap-1" {
                @for opt in opts.options {
                    label class="label text-sm cursor-pointer justify-start gap-2 flex flex-row items-center" {
                        (PreEscaped(format!(
                            r#"<input type="radio" name="{}" value="{}" class="radio"{}"{}>"#,
                            escape_attr(opts.name),
                            escape_attr(opt.value),
                            if opt.checked { " checked" } else { "" },
                            opts.attrs.as_string()
                        )))
                        span class="label-text" { (opt.label) }
                    }
                }
            }
        }
    }
}

/// One option in an [`InputSelect`].
pub struct InputSelectOption<'a> {
    pub value: &'a str,
    pub label: &'a str,
    pub selected: bool,
}

/// Dropdown select from static options.
pub struct InputSelect<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub required: bool,
    pub classes: &'a str,
    pub options: &'a [InputSelectOption<'a>],
    pub attrs: HtmlAttrs,
}

impl Default for InputSelect<'_> {
    fn default() -> Self {
        Self {
            label: "",
            name: "",
            required: false,
            classes: "",
            options: &[],
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a select input.
pub fn input_select(opts: InputSelect<'_>) -> Markup {
    let required_attr = if opts.required { " required" } else { "" };
    html! {
        div class=(format!("my-1 {}", opts.classes)) {
            label class="label text-sm font-bold flex flex-col items-start gap-1" {
                (opts.label)
                (PreEscaped(format!(
                    r#"<select name="{}" class="{}"{}{}>"#,
                    escape_attr(opts.name),
                    escape_attr(&format!("select select-bordered w-full {}", opts.classes)),
                    required_attr,
                    opts.attrs.as_string()
                )))
                @for opt in opts.options {
                    option value=(opt.value) selected[opt.selected] { (opt.label) }
                }
                (PreEscaped("</select>"))
            }
        }
    }
}

/// Telephone input (`type="tel"`).
pub struct InputPhone<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub value: &'a str,
    pub required: bool,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputPhone<'_> {
    fn default() -> Self {
        Self {
            label: "Phone",
            name: "phone",
            value: "",
            required: false,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a phone input.
pub fn input_phone(opts: InputPhone<'_>) -> Markup {
    let wrap = format!("my-1 {}", opts.classes);
    let input_class = format!("input input-bordered w-full {}", opts.classes);
    labeled_input(LabeledInput {
        wrap_class: &wrap,
        label: opts.label,
        show_label: true,
        input_type: "tel",
        name: opts.name,
        value: opts.value,
        input_class: &input_class,
        required: opts.required,
        attrs: &opts.attrs,
    })
}

/// Date text input (`DD/MM/YYYY`). Native `type="date"` is not used.
pub struct InputDate<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub value: &'a str,
    pub required: bool,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputDate<'_> {
    fn default() -> Self {
        Self {
            label: "Date",
            name: "",
            value: "",
            required: false,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a date input.
pub fn input_date(opts: InputDate<'_>) -> Markup {
    let wrap = format!("my-1 {}", opts.classes);
    let input_class = format!("input input-bordered w-full {}", opts.classes);
    let attrs = HtmlAttrs::new()
        .set("placeholder", "DD/MM/YYYY")
        .set("autocomplete", "off")
        .merge(&opts.attrs);
    labeled_input(LabeledInput {
        wrap_class: &wrap,
        label: opts.label,
        show_label: true,
        input_type: "text",
        name: opts.name,
        value: opts.value,
        input_class: &input_class,
        required: opts.required,
        attrs: &attrs,
    })
}

/// Time picker input (`type="time"`).
pub struct InputTime<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub value: &'a str,
    pub required: bool,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputTime<'_> {
    fn default() -> Self {
        Self {
            label: "Time",
            name: "",
            value: "",
            required: false,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a time input.
pub fn input_time(opts: InputTime<'_>) -> Markup {
    let wrap = format!("my-1 {}", opts.classes);
    let input_class = format!("input input-bordered w-full {}", opts.classes);
    labeled_input(LabeledInput {
        wrap_class: &wrap,
        label: opts.label,
        show_label: true,
        input_type: "time",
        name: opts.name,
        value: opts.value,
        input_class: &input_class,
        required: opts.required,
        attrs: &opts.attrs,
    })
}

/// Datetime text input (`DD/MM/YYYY HH:MM:SS`). Native `datetime-local` is not used.
pub struct InputDatetime<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub value: &'a str,
    pub required: bool,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputDatetime<'_> {
    fn default() -> Self {
        Self {
            label: "Date & time",
            name: "",
            value: "",
            required: false,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a datetime input.
pub fn input_datetime(opts: InputDatetime<'_>) -> Markup {
    let wrap = format!("my-1 {}", opts.classes);
    let input_class = format!("input input-bordered w-full {}", opts.classes);
    let attrs = HtmlAttrs::new()
        .set("placeholder", "DD/MM/YYYY HH:MM:SS")
        .set("autocomplete", "off")
        .merge(&opts.attrs);
    labeled_input(LabeledInput {
        wrap_class: &wrap,
        label: opts.label,
        show_label: true,
        input_type: "text",
        name: opts.name,
        value: opts.value,
        input_class: &input_class,
        required: opts.required,
        attrs: &attrs,
    })
}

/// Flexible duration text input (e.g. `"2 months 3 days 5 seconds"`, `"720h"`).
pub struct InputDuration<'a> {
    pub label: &'a str,
    pub name: &'a str,
    /// Initial duration string.
    pub value: &'a str,
    pub required: bool,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputDuration<'_> {
    fn default() -> Self {
        Self {
            label: "Duration",
            name: "",
            value: "",
            required: false,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a duration text input with unit hint.
pub fn input_duration(opts: InputDuration<'_>) -> Markup {
    let wrap = format!("my-1 {}", opts.classes);
    let input_class = format!("input input-bordered w-full {}", opts.classes);
    let required_attr = if opts.required { " required" } else { "" };
    html! {
        div class=(wrap) {
            label class="label text-sm font-bold flex flex-col items-start gap-1" {
                (opts.label)
                (PreEscaped(format!(
                    r#"<input type="text" name="{}" value="{}" placeholder="e.g. 2 months 3 days 5 seconds" class="{}"{}{}>"#,
                    escape_attr(opts.name),
                    escape_attr(opts.value),
                    escape_attr(&input_class),
                    required_attr,
                    opts.attrs.as_string(),
                )))
                span class="text-xs text-base-content/60 mt-1" {
                    "Use units like seconds, minutes, hours, days, weeks, months, years — commas optional."
                }
            }
        }
    }
}

/// File upload control with optional `accept` and `multiple`.
pub struct InputFile<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub required: bool,
    pub multiple: bool,
    /// HTML `accept` attribute (e.g. `"image/*"`, `".zip"`); empty allows any file.
    pub accept: &'a str,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputFile<'_> {
    fn default() -> Self {
        Self {
            label: "",
            name: "",
            required: false,
            multiple: false,
            accept: "",
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a file input.
pub fn input_file(opts: InputFile<'_>) -> Markup {
    let required_attr = if opts.required { " required" } else { "" };
    let multiple_attr = if opts.multiple { " multiple" } else { "" };
    let accept_attr = if opts.accept.is_empty() {
        String::new()
    } else {
        format!(r#" accept="{}""#, escape_attr(opts.accept))
    };
    html! {
        div class=(format!("my-1 {}", opts.classes)) {
            label class="label text-sm font-bold flex flex-col items-start gap-1" {
                (opts.label)
                (PreEscaped(format!(
                    r#"<input type="file" name="{}" class="{}"{}{}{}{}>"#,
                    escape_attr(opts.name),
                    escape_attr(&format!("file-input file-input-bordered w-full {}", opts.classes)),
                    required_attr,
                    multiple_attr,
                    accept_attr,
                    opts.attrs.as_string()
                )))
            }
        }
    }
}

use crate::components::htmx::{
    FK_DROPDOWN_ID_PREFIX, HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL,
};
use crate::components::text::icon;

/// Foreign-key picker: searchable combobox plus a button that opens the selection table.
pub struct InputForeignKey<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub value: &'a str,
    pub display: &'a str,
    pub placeholder: &'a str,
    pub url: &'a str,
    /// Optional compile-time region id for the FK widget root.
    pub uid: &'a str,
    /// Query parameter name for typeahead search (picker list filter). Default `Name`.
    pub search_key: &'a str,
    pub required: bool,
    pub hidden: bool,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputForeignKey<'_> {
    fn default() -> Self {
        Self {
            label: "",
            name: "",
            value: "",
            display: "",
            placeholder: "Select...",
            url: "",
            uid: "",
            search_key: "Name",
            required: false,
            hidden: false,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

fn fk_dropdown_id(uid: &str, name: &str) -> String {
    let raw = if !uid.is_empty() { uid } else { name };
    let sanitized: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("{FK_DROPDOWN_ID_PREFIX}{sanitized}")
}

fn json_str(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
}

/// Render an FK picker: typeahead search box, table-open button, HTMX modal.
pub fn input_foreign_key(opts: InputForeignKey<'_>) -> Markup {
    if opts.hidden {
        return html! {
            div class=(format!("my-1 hidden {}", opts.classes)) {
                (PreEscaped(format!(
                    r#"<input type="hidden" name="{}" value="{}"{}>"#,
                    escape_attr(opts.name),
                    escape_attr(opts.value),
                    opts.attrs.as_string()
                )))
            }
        };
    }

    let placeholder = if opts.placeholder.is_empty() {
        "Select..."
    } else {
        opts.placeholder
    };
    let search_key = if opts.search_key.is_empty() {
        "Name"
    } else {
        opts.search_key
    };
    let mut url = opts.url.to_string();
    if !opts.name.is_empty() {
        let sep = if url.contains('?') { '&' } else { '?' };
        url = format!("{url}{sep}target_input={}", opts.name);
    }
    let dropdown_id = fk_dropdown_id(opts.uid, opts.name);
    let search_id = format!("{dropdown_id}-q");
    let alpine_data = format!(
        r#"{{
            value: {value},
            display: {display},
            query: {display},
            placeholder: {placeholder},
            fieldName: {name},
            open: false,
            pendingCreate: false,
            hasCreate: false,
            isVisible() {{ return true }},
            visibleCount() {{ return 99 }},
            init() {{
                this.$nextTick(() => {{
                    const results = this.$refs.results
                    if (!results || typeof MutationObserver === 'undefined') {{
                        return
                    }}
                    new MutationObserver(() => this.relocateCreate()).observe(results, {{
                        childList: true,
                        subtree: true,
                    }})
                }})
            }},
            applySelect(detail) {{
                if (!detail || detail.name !== {name}) {{
                    return
                }}
                this.value = detail.value
                this.display = detail.display ? String(detail.display) : ''
                this.query = this.display
                this.open = false
                this.pendingCreate = false
            }},
            clear() {{
                this.value = ''
                this.display = ''
                this.query = ''
                this.open = false
                this.pendingCreate = false
            }},
            closeOutside() {{
                this.open = false
                this.query = this.display || ''
            }},
            createButton() {{
                const results = this.$refs.results
                return results ? results.querySelector('.fk-modal-host button') : null
            }},
            relocateCreate() {{
                const results = this.$refs.results
                if (results) {{
                    results.querySelectorAll('form').forEach((f) => f.remove())
                    results.querySelectorAll('details.dropdown').forEach((d) => d.remove())
                }}
                const btn = this.createButton()
                this.hasCreate = !!btn
                if (!btn) {{
                    return
                }}
                const raw = btn.getAttribute('hx-get') || ''
                try {{
                    const u = new URL(raw, window.location.href)
                    if (this.query && String(this.query).trim()) {{
                        u.searchParams.set('Name', String(this.query).trim())
                    }}
                    if (this.fieldName) {{
                        u.searchParams.set('target_input', this.fieldName)
                    }}
                    btn.setAttribute('hx-get', u.pathname + u.search + u.hash)
                }} catch (e) {{}}
                if (window.htmx) {{
                    window.htmx.process(btn)
                }}
            }},
            openCreate() {{
                this.relocateCreate()
                const btn = this.createButton()
                if (!btn) {{
                    return
                }}
                this.pendingCreate = true
                this.open = false
                btn.click()
            }},
            onCreated(detail) {{
                if (!detail) {{
                    return
                }}
                const name = detail.name || (this.pendingCreate ? this.fieldName : '')
                if (!name || name !== this.fieldName) {{
                    return
                }}
                this.applySelect({{
                    name: this.fieldName,
                    value: detail.value,
                    display: detail.display,
                }})
                document.querySelectorAll('dialog.fk-modal-container').forEach((d) => {{
                    if (d.querySelector('.fk-picker-results')) {{
                        return
                    }}
                    if (d.querySelector('.data-table-container')) {{
                        d.remove()
                    }}
                }})
            }}
        }}"#,
        value = json_str(opts.value),
        display = json_str(opts.display),
        placeholder = json_str(placeholder),
        name = json_str(opts.name),
    );
    let required_attr = if opts.required { " required" } else { "" };
    let id_attr = if opts.uid.is_empty() {
        String::new()
    } else {
        format!(r#" id="{}""#, escape_attr(opts.uid))
    };
    let dropdown_target = format!("#{dropdown_id}");
    let search_include = format!("#{search_id}");
    let search_attrs = HtmlAttrs::new()
        .set("id", &search_id)
        .set("type", "search")
        .set("class", "input input-bordered join-item flex-1 min-w-0")
        .set("form", "fk-picker-search")
        .set("name", search_key)
        .set("x-model", "query")
        .set(":placeholder", "placeholder")
        .set("autocomplete", "off")
        .set("autocorrect", "off")
        .set("spellcheck", "false")
        .set("role", "combobox")
        .set("aria-autocomplete", "list")
        .set("aria-controls", &dropdown_id)
        .set(":aria-expanded", "open")
        .set("hx-get", &url)
        .set(
            "hx-trigger",
            "input changed delay:300ms[this.value.trim() !== '']",
        )
        .set("hx-target", &dropdown_target)
        .set("hx-swap", "innerHTML")
        .set("hx-push-url", "false")
        .set("hx-sync", "this:replace")
        .set(
            "@input",
            "open = query.trim() !== ''; pendingCreate = false",
        )
        .set(
            "hx-on::after:swap",
            "var d=window.Alpine&&Alpine.$data(this.closest('[x-data]'));if(d&&d.relocateCreate){d.relocateCreate();d.open=true}",
        )
        .set("@keydown.enter.prevent", "")
        .set("@keydown.escape", "open = false; query = display || ''");
    let table_btn_attrs = HtmlAttrs::new()
        .set("type", "button")
        .set("class", "btn btn-square join-item")
        .set("hx-get", &url)
        .set("hx-target", HTMX_TARGET_BODY_MODAL)
        .set("hx-swap", HTMX_SWAP_BODY_MODAL)
        .set("hx-push-url", "false")
        .set("hx-include", &search_include)
        .set("@click", "open = false; pendingCreate = false")
        .set("aria-label", "Open selection table");
    let results_attrs = HtmlAttrs::new()
        .set("id", &dropdown_id)
        .set(
            "class",
            "fk-picker-results overflow-auto min-h-0 [&_thead]:hidden [&_.text-xl]:hidden [&_select.select]:hidden [&_details.dropdown]:hidden [&_.join]:hidden [&_.flex.justify-between.items-center.my-2]:hidden [&_.table-container]:border-0 [&_.table-container]:rounded-none",
        )
        .set("x-ref", "results");
    let panel_attrs = HtmlAttrs::new()
        .set(
            "class",
            "absolute left-0 right-0 z-50 mt-1 max-h-72 flex flex-col overflow-hidden rounded-box border border-base-300 bg-base-100 shadow",
        )
        .set("x-show", "open")
        .set("x-cloak", "");

    html! {
        (PreEscaped(format!(
            r#"<div{} class="my-1 relative w-full {}" x-data="{}" @fk-select.window="applySelect($event.detail)" @lariv-fk-created.window="onCreated($event.detail)" @click.outside="closeOutside()">"#,
            id_attr,
            escape_attr(opts.classes),
            escape_attr(&alpine_data),
        )))
        label class="label text-sm font-bold flex flex-col items-start gap-1 w-full" {
            (opts.label)
            (PreEscaped(format!(
                r#"<input type="hidden" name="{}" :value="value"{}{}>"#,
                escape_attr(opts.name),
                required_attr,
                opts.attrs.as_string()
            )))
            div class="join w-full" {
                (PreEscaped(format!("<input{}>", search_attrs.as_string())))
                (PreEscaped(format!("<button{}>", table_btn_attrs.as_string())))
                (icon("table-cells", ""))
                (PreEscaped("</button>"))
                @if !opts.required {
                    (PreEscaped(
                        r#"<button type="button" class="btn btn-ghost btn-square join-item" @click.stop="clear()" x-show="value" aria-label="Clear selection">"#
                    ))
                    (icon("x-mark", ""))
                    (PreEscaped("</button>"))
                }
            }
        }
        (PreEscaped(format!("<div{}>", panel_attrs.as_string())))
        (PreEscaped(format!("<div{}></div>", results_attrs.as_string())))
        (PreEscaped(
            r#"<button type="button" class="btn btn-ghost btn-sm w-full justify-start rounded-none shrink-0 border-t border-base-300" x-ref="createFooter" x-show="hasCreate" x-cloak @click.stop="openCreate()">Create New…</button>"#,
        ))
        (PreEscaped("</div>"))
        (PreEscaped("</div>"))
    }
}

/// Selected chip for [`InputManyToMany`].
#[derive(Clone, Debug, serde::Serialize)]
pub struct ManyToManyItem {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "Value")]
    pub value: String,
    #[serde(rename = "Color", skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl ManyToManyItem {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            color: None,
        }
    }

    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        let color = color.into();
        self.color = (!color.is_empty()).then_some(color);
        self
    }
}

/// Many-to-many picker that opens a multi-select modal.
pub struct InputManyToMany<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub items: &'a [ManyToManyItem],
    pub placeholder: &'a str,
    pub url: &'a str,
    pub required: bool,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputManyToMany<'_> {
    fn default() -> Self {
        Self {
            label: "",
            name: "",
            items: &[],
            placeholder: "Select...",
            url: "",
            required: false,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a many-to-many chip picker with HTMX modal.
pub fn input_many_to_many(opts: InputManyToMany<'_>) -> Markup {
    let placeholder = if opts.placeholder.is_empty() {
        "Select..."
    } else {
        opts.placeholder
    };
    let mut url = opts.url.to_string();
    if !opts.name.is_empty() {
        let sep = if url.contains('?') { '&' } else { '?' };
        url = format!("{url}{sep}target_input={}", opts.name);
    }
    let items_json = serde_json::to_string(opts.items).unwrap_or_else(|_| "[]".into());
    let name_json = serde_json::to_string(opts.name).unwrap_or_else(|_| "\"\"".into());
    let placeholder_json = serde_json::to_string(placeholder).unwrap_or_else(|_| "\"\"".into());
    let alpine_data = format!(
        r#"{{
		items: {items_json},
		placeholder: {placeholder_json},
		syncStore() {{
			if (typeof Alpine === 'undefined') {{
				return
			}}
			if (!Alpine.store('m2mSelections')) {{
				Alpine.store('m2mSelections', {{}})
			}}
			Alpine.store('m2mSelections')[{name_json}] = this.items
		}},
		hasItem(value) {{
			value = String(value)
			return this.items.some(item => item.Key === value)
		}},
		addItem(detail) {{
			const value = String(detail.value)
			if (this.hasItem(value)) {{
				return
			}}
			const display = detail.display ? String(detail.display) : value
			const color = detail.color || detail.Color || ''
			const item = {{ Key: value, Value: display }}
			if (color) {{
				item.Color = String(color)
			}}
			this.items = [...this.items, item]
			this.syncStore()
		}},
		contrastText(hex) {{
			const h = String(hex || '').replace('#', '')
			if (h.length !== 6) {{
				return '#ffffff'
			}}
			const toLin = (c) => c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4)
			const r = toLin(parseInt(h.slice(0, 2), 16) / 255)
			const g = toLin(parseInt(h.slice(2, 4), 16) / 255)
			const b = toLin(parseInt(h.slice(4, 6), 16) / 255)
			const L = 0.2126 * r + 0.7152 * g + 0.0722 * b
			return L > 0.179 ? '#111827' : '#ffffff'
		}},
		removeItem(detail) {{
			const value = String(detail.value)
			this.items = this.items.filter(item => item.Key !== value)
			this.syncStore()
		}},
		eventHandler(ev) {{
			if (ev.detail.name === {name_json}) {{
				if (!this.hasItem(ev.detail.value)) {{
					this.addItem(ev.detail)
				}} else {{
					this.removeItem(ev.detail)
				}}
			}}
		}},
		onCreated(detail) {{
			if (!detail || detail.name !== {name_json}) {{
				return
			}}
			this.addItem(detail)
			document.querySelectorAll('dialog.fk-modal-container').forEach((d) => {{
				if (d.querySelector('.fk-picker-results')) {{
					return
				}}
				if (d.querySelector('.data-table-container')) {{
					d.remove()
				}}
			}})
		}}
	}}"#
    );

    html! {
        (PreEscaped(format!(
            r#"<div class="my-1 relative {}" x-data="{}" x-init="syncStore()" @fk-multi-select.window="eventHandler($event)" @lariv-fk-created.window="onCreated($event.detail)"{}>"#,
            escape_attr(opts.classes),
            escape_attr(&alpine_data),
            opts.attrs.as_string()
        )))
        div class="flex flex-col items-start gap-1" {
            @if !opts.label.is_empty() {
                label class="label text-sm font-bold" { (opts.label) }
            }
            (PreEscaped(format!(
                r#"<div class="input input-bordered w-full min-h-12 h-auto flex flex-wrap items-center gap-2 cursor-pointer" :class="items.length ? '' : 'opacity-50'" hx-get="{}" hx-target="{}" hx-swap="{}" hx-push-url="false">"#,
                escape_attr(&url),
                HTMX_TARGET_BODY_MODAL,
                HTMX_SWAP_BODY_MODAL
            )))
            span x-show="items.length === 0" x-text="placeholder" {}
            template x-for="item in items" x-bind:key="item.Key" {
                (PreEscaped(
                    r#"<div class="flex items-center gap-1 rounded-lg pl-2 pr-1 py-1" :class="item.Color ? 'border-0 font-medium' : 'bg-base-200'" :style="item.Color ? { backgroundColor: item.Color, color: contrastText(item.Color) } : {}" @click="$event.stopPropagation()">"#
                ))
                (PreEscaped(format!(
                    r#"<input type="hidden" name="{}" :value="item.Key">"#,
                    escape_attr(opts.name)
                )))
                span class="text-sm flex-1 min-w-0 truncate" x-text="item.Value" {}
                (PreEscaped(
                    r#"<button type="button" class="btn btn-ghost btn-square btn-xs shrink-0" :class="item.Color && 'text-current hover:bg-black/10'" @click.stop="removeItem({ value: item.Key })" aria-label="Remove">"#
                ))
                (icon("x-mark", ""))
                (PreEscaped("</button>"))
                (PreEscaped("</div>"))
            }
            (PreEscaped("</div>"))
        }
        (PreEscaped("</div>"))
    }
}
