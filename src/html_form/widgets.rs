//! Stock [`FormWidget`] implementations shipped with lariv-rs.
//!
//! App code can define its own types the same way — the renderer does not
//! special-case these. Reference a widget in `#[widget(Text)]` etc.

use maud::Markup;

use crate::components::{
    CodeEditorInput, FieldText, HtmlAttrs, InputCheckbox, InputColor, InputDate, InputDatetime,
    InputDuration, InputEmail, InputFile, InputForeignKey, InputList, InputManyToMany, InputNumber,
    InputPassword, InputPhone, InputSelect, InputSelectOption, InputText, InputTextarea,
    code_editor_input, field_text, input_checkbox, input_color, input_date, input_datetime,
    input_duration, input_email, input_file, input_foreign_key, input_list, input_many_to_many,
    input_number, input_password, input_phone, input_select, input_text, input_textarea,
};
use crate::html_form::{FieldRender, FormCtx, FormWidget};

/// Single-line text input widget (supports Alpine `x-model` via field spec).
pub struct Text;
impl FormWidget for Text {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let attrs = match field.spec.model {
            Some(m) => HtmlAttrs::new().set("x-model", m),
            None => HtmlAttrs::new(),
        };
        input_text(InputText {
            label: field.label,
            name: field.name,
            value: field.value,
            required: field.required,
            attrs,
            ..Default::default()
        })
    }
}

/// Multi-line textarea widget (supports Alpine `x-model` via field spec).
pub struct Textarea;
impl FormWidget for Textarea {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let attrs = match field.spec.model {
            Some(m) => HtmlAttrs::new().set("x-model", m),
            None => HtmlAttrs::new(),
        };
        input_textarea(InputTextarea {
            label: field.label,
            name: field.name,
            value: field.value,
            required: field.required,
            rows: field.spec.rows.unwrap_or(3),
            attrs,
            ..Default::default()
        })
    }
}

/// CodeMirror 6 code editor widget (plaintext by default).
pub struct CodeEditor;
impl FormWidget for CodeEditor {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        code_editor_input(CodeEditorInput {
            label: field.label,
            name: field.name,
            value: field.value,
            required: field.required,
            rows: field.spec.rows.unwrap_or(12),
            language: "plaintext",
            ..Default::default()
        })
    }
}

/// Email input widget.
pub struct Email;
impl FormWidget for Email {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_email(InputEmail {
            label: field.label,
            name: field.name,
            value: field.value,
            required: field.required,
            ..Default::default()
        })
    }
}

/// HTML color picker (`#rrggbb`).
pub struct Color;
impl FormWidget for Color {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_color(InputColor {
            label: field.label,
            name: field.name,
            value: field.value,
            required: field.required,
            ..Default::default()
        })
    }
}

/// Password input widget.
pub struct Password;
impl FormWidget for Password {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_password(InputPassword {
            label: field.label,
            name: field.name,
            value: field.value,
            required: field.required,
            ..Default::default()
        })
    }
}

/// Phone input widget.
pub struct Phone;
impl FormWidget for Phone {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_phone(InputPhone {
            label: field.label,
            name: field.name,
            value: field.value,
            required: field.required,
            ..Default::default()
        })
    }
}

/// Numeric input widget.
pub struct Number;
impl FormWidget for Number {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_number(InputNumber {
            label: field.label,
            name: field.name,
            value: field.value,
            required: field.required,
            ..Default::default()
        })
    }
}

/// Checkbox widget (supports Alpine `x-model` via field spec).
pub struct Checkbox;
impl FormWidget for Checkbox {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let attrs = match field.spec.model {
            Some(m) => HtmlAttrs::new().set("x-model", m),
            None => HtmlAttrs::new(),
        };
        input_checkbox(InputCheckbox {
            label: field.label,
            name: field.name,
            checked: ctx.checked_of(field.name),
            attrs,
            ..Default::default()
        })
    }
}

/// Select dropdown; choices come from [`FormCtx::choices`].
///
/// Supports Alpine `x-model` via the field spec `model` attribute (requires [`FormCtx::x_data`]).
pub struct Select;
impl FormWidget for Select {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let key = field.spec.choices_key.unwrap_or(field.name);
        let choices = ctx.choices_of(key);
        let mut options: Vec<InputSelectOption<'_>> = Vec::with_capacity(choices.len() + 1);
        // Optional selects get a blank "None" choice; required selects must pick a real value.
        if !field.required {
            options.push(InputSelectOption {
                value: "",
                label: "None",
                selected: field.value.is_empty(),
            });
        }
        for (id, label) in choices {
            options.push(InputSelectOption {
                value: id.as_str(),
                label: label.as_str(),
                selected: field.value == id.as_str(),
            });
        }
        let attrs = match field.spec.model {
            Some(m) => HtmlAttrs::new().set("x-model", m),
            None => HtmlAttrs::new(),
        };
        input_select(InputSelect {
            label: field.label,
            name: field.name,
            required: field.required,
            options: &options,
            attrs,
            ..Default::default()
        })
    }
}

/// Date text input (`DD/MM/YYYY`) with a trailing button that opens the native date picker.
pub struct Date;
impl FormWidget for Date {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_date(InputDate {
            label: field.label,
            name: field.name,
            value: field.value,
            required: field.required,
            ..Default::default()
        })
    }
}

/// Datetime text input (`DD/MM/YYYY HH:MM:SS`) with a trailing button that opens the picker
/// (supports Alpine `x-model` via field spec).
pub struct Datetime;
impl FormWidget for Datetime {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let attrs = match field.spec.model {
            Some(m) => HtmlAttrs::new().set("x-model", m),
            None => HtmlAttrs::new(),
        };
        input_datetime(InputDatetime {
            label: field.label,
            name: field.name,
            value: field.value,
            required: field.required,
            attrs,
            ..Default::default()
        })
    }
}

/// Go-duration picker widget — flexible text input parsed by [`crate::duration::parse_duration`].
pub struct Duration;
impl FormWidget for Duration {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_duration(InputDuration {
            label: field.label,
            name: field.name,
            value: field.value,
            required: field.required,
            ..Default::default()
        })
    }
}

/// Foreign-key picker: typeahead search plus HTMX selection-table modal
/// (choices from [`FormCtx::url`] / [`FormCtx::display`]).
pub struct ForeignKey;
impl FormWidget for ForeignKey {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let display_key = field.spec.display_key.unwrap_or(field.name);
        let ph = field.spec.placeholder.unwrap_or("Select...");
        input_foreign_key(InputForeignKey {
            label: field.label,
            name: field.name,
            value: field.value,
            display: ctx.display_of(display_key),
            placeholder: ph,
            url: ctx.url_of(field.spec),
            uid: field.spec.swap_key.unwrap_or(""),
            required: field.required,
            ..Default::default()
        })
    }
}

/// Many-to-many chip picker (items from [`FormCtx::m2m`]).
pub struct ManyToMany;
impl FormWidget for ManyToMany {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let ph = field.spec.placeholder.unwrap_or("Select...");
        let attrs = match field.spec.swap_key {
            Some(id) => HtmlAttrs::new().set("id", id),
            None => HtmlAttrs::new(),
        };
        input_many_to_many(InputManyToMany {
            label: field.label,
            name: field.name,
            items: ctx.m2m_of(field.name),
            placeholder: ph,
            url: ctx.url_of(field.spec),
            attrs,
            ..Default::default()
        })
    }
}

/// Freeform string list of editable rows (items from [`FormCtx::list`], else newline-split value).
pub struct List;
impl FormWidget for List {
    fn render(ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let ph = field.spec.placeholder.unwrap_or("Add item...");
        let from_ctx = ctx.list_of(field.name);
        let from_value: Vec<String> = if from_ctx.is_empty() && !field.value.is_empty() {
            field
                .value
                .split(|c: char| c == '\n' || c == '\r' || c == ',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        } else {
            Vec::new()
        };
        let items = if from_ctx.is_empty() {
            from_value.as_slice()
        } else {
            from_ctx
        };
        input_list(InputList {
            label: field.label,
            name: field.name,
            items,
            placeholder: ph,
            required: field.required,
            normalize_subreddit: true,
            ..Default::default()
        })
    }
}

/// File upload widget (`Upload` field type on submit).
pub struct File;
impl FormWidget for File {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_file(InputFile {
            label: field.label,
            name: field.name,
            required: field.required,
            multiple: field.spec.multiple,
            accept: field.spec.accept.unwrap_or(""),
            ..Default::default()
        })
    }
}

/// Marker widget for tagged-enum fields — the macro expands to [`crate::html_form::render_kind`].
pub struct Kind;

/// Section heading (non-input label divider).
pub struct Section;
impl FormWidget for Section {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        field_text(FieldText {
            value: field.label,
            classes: "text-lg font-semibold mt-4",
        })
    }
}
