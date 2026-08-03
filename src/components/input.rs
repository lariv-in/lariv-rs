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

/// Date picker input (`type="date"`).
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
    labeled_input(LabeledInput {
        wrap_class: &wrap,
        label: opts.label,
        show_label: true,
        input_type: "date",
        name: opts.name,
        value: opts.value,
        input_class: &input_class,
        required: opts.required,
        attrs: &opts.attrs,
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

/// Local datetime picker (`type="datetime-local"`).
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
    labeled_input(LabeledInput {
        wrap_class: &wrap,
        label: opts.label,
        show_label: true,
        input_type: "datetime-local",
        name: opts.name,
        value: opts.value,
        input_class: &input_class,
        required: opts.required,
        attrs: &opts.attrs,
    })
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

use crate::components::htmx::{HTMX_SELECT_UNSET, HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL};
use crate::components::text::icon;

/// Foreign-key picker that opens a selection modal.
pub struct InputForeignKey<'a> {
    pub label: &'a str,
    pub name: &'a str,
    pub value: &'a str,
    pub display: &'a str,
    pub placeholder: &'a str,
    pub url: &'a str,
    /// Optional compile-time region id for the FK widget root.
    pub uid: &'a str,
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
            required: false,
            hidden: false,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render an FK picker with HTMX modal and Alpine display state.
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
    let mut url = opts.url.to_string();
    if !opts.name.is_empty() {
        let sep = if url.contains('?') { '&' } else { '?' };
        url = format!("{url}{sep}target_input={}", opts.name);
    }
    let alpine = serde_json::json!({
        "value": opts.value,
        "display": opts.display,
        "placeholder": placeholder,
    });
    let alpine_data = alpine.to_string();
    let event_handler = format!(
        "if ($event.detail.name === '{}') {{ value = $event.detail.value; display = $event.detail.display }}",
        opts.name.replace('\'', "\\'")
    );
    let required_attr = if opts.required { " required" } else { "" };
    let id_attr = if opts.uid.is_empty() {
        String::new()
    } else {
        format!(r#" id="{}""#, escape_attr(opts.uid))
    };

    html! {
        (PreEscaped(format!(
            r#"<div{} class="my-1 relative {}" x-data="{}" @fk-select.window="{}">"#,
            id_attr,
            escape_attr(opts.classes),
            escape_attr(&alpine_data),
            escape_attr(&event_handler)
        )))
        label class="label text-sm font-bold flex flex-col items-start gap-1" {
            (opts.label)
            (PreEscaped(format!(
                r#"<input type="hidden" name="{}" :value="value"{}{}>"#,
                escape_attr(opts.name),
                required_attr,
                opts.attrs.as_string()
            )))
            div class="flex w-full items-stretch gap-1" {
                (PreEscaped(format!(
                    r#"<div class="input input-bordered flex-1 flex items-center cursor-pointer" :class="display ? '' : 'opacity-50'" hx-get="{}" hx-target="{}" hx-select="{}" hx-swap="{}" hx-push-url="false">"#,
                    escape_attr(&url),
                    HTMX_TARGET_BODY_MODAL,
                    HTMX_SELECT_UNSET,
                    HTMX_SWAP_BODY_MODAL
                )))
                span x-text="display || placeholder" {}
                (PreEscaped("</div>"))
                @if !opts.required {
                    (PreEscaped(
                        r#"<button type="button" class="btn btn-ghost btn-square shrink-0" @click.stop="value = ''; display = ''" x-show="value" aria-label="Clear selection">"#
                    ))
                    (icon("x-mark", ""))
                    (PreEscaped("</button>"))
                }
            }
        }
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
			this.items = [...this.items, {{ Key: value, Value: display }}]
			this.syncStore()
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
		}}
	}}"#
    );

    html! {
        (PreEscaped(format!(
            r#"<div class="my-1 relative {}" x-data="{}" x-init="syncStore()" @fk-multi-select.window="eventHandler($event)"{}>"#,
            escape_attr(opts.classes),
            escape_attr(&alpine_data),
            opts.attrs.as_string()
        )))
        div class="flex flex-col items-start gap-1" {
            @if !opts.label.is_empty() {
                label class="label text-sm font-bold" { (opts.label) }
            }
            (PreEscaped(format!(
                r#"<div class="input input-bordered w-full min-h-12 h-auto flex flex-wrap items-center gap-2 cursor-pointer" :class="items.length ? '' : 'opacity-50'" hx-get="{}" hx-target="{}" hx-select="{}" hx-swap="{}" hx-push-url="false">"#,
                escape_attr(&url),
                HTMX_TARGET_BODY_MODAL,
                HTMX_SELECT_UNSET,
                HTMX_SWAP_BODY_MODAL
            )))
            span x-show="items.length === 0" x-text="placeholder" {}
            template x-for="item in items" x-bind:key="item.Key" {
                (PreEscaped(
                    r#"<div class="flex items-center gap-1 rounded-lg bg-base-200 pl-2 pr-1 py-1" @click="$event.stopPropagation()">"#
                ))
                (PreEscaped(format!(
                    r#"<input type="hidden" name="{}" :value="item.Key">"#,
                    escape_attr(opts.name)
                )))
                span class="text-sm flex-1 min-w-0 truncate" x-text="item.Value" {}
                (PreEscaped(
                    r#"<button type="button" class="btn btn-ghost btn-square btn-xs shrink-0" @click.stop="removeItem({ value: item.Key })" aria-label="Remove">"#
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
