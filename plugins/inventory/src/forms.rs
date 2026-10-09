use lariv_core::components::{
    CodeEditorInput, HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL, InputDatetime, InputSelect,
    InputSelectOption, code_editor_input, input_datetime, input_select, label_hint,
};
use lariv_core::html_form::{
    FieldRender, FormCtx, FormWidget, html_form,
    widgets::{Checkbox, Select, Text, Textarea},
};
use lariv_core::length::LengthUnit;
use lariv_plugin_contacts::routes::{CompanyFkSelectRouteTag, ContactFkSelectRouteTag};
use lariv_plugin_filesystem::routes::VNodeFileSelectRouteTag;
use maud::{Markup, html};

use crate::logic::pdf::{DEFAULT_MOVEMENT_IN_TEMPLATE, DEFAULT_MOVEMENT_OUT_TEMPLATE};
use crate::logic::qty::WeightUnit;
use crate::movement_lines::{InputMovementLines, input_movement_lines};
use crate::routes::{MovementPdfPreviewInRouteTag, MovementPdfPreviewOutRouteTag};

#[html_form]
pub struct StockForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(
        label = "Company",
        required,
        widget = ForeignKey,
        route = CompanyFkSelectRouteTag,
        swap_key = "inventory-stock-company",
        display = "company",
        placeholder = "Select company…"
    )]
    pub company_id: i64,

    #[form(
        label = "Quantity type",
        required,
        widget = Select,
        choices = "qty_type",
        model = "qtyType"
    )]
    pub qty_type: String,

    #[form(
        label = "Quantity unit",
        widget = QtyUnitSelect,
        show = "qtyType === 'length' || qtyType === 'weight'"
    )]
    pub qty_unit: String,
}

/// Unit dropdown that follows the quantity type: length units, or weight units.
pub struct QtyUnitSelect;

impl FormWidget for QtyUnitSelect {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let length: Vec<(&str, &str)> = LengthUnit::ALL
            .iter()
            .map(|unit| (unit.as_str(), unit.as_str()))
            .collect();
        let weight: Vec<(&str, &str)> = WeightUnit::ALL
            .iter()
            .map(|unit| (unit.as_str(), unit.as_str()))
            .collect();
        html! {
            (unit_fieldset(
                "qtyType === 'length'",
                "qtyType !== 'length'",
                unit_select(field.label, field.name, &length, field.value),
            ))
            (unit_fieldset(
                "qtyType === 'weight'",
                "qtyType !== 'weight'",
                unit_select(field.label, field.name, &weight, field.value),
            ))
        }
    }
}

fn unit_fieldset(show: &str, disabled: &str, body: Markup) -> Markup {
    html! {
        fieldset class="border-0 p-0 m-0 min-w-0" x-show=(show) x-cloak x-bind:disabled=(disabled) {
            (body)
        }
    }
}

fn unit_select(label: &str, name: &str, units: &[(&str, &str)], current: &str) -> Markup {
    let matched = units.iter().any(|(value, _)| *value == current);
    let options: Vec<InputSelectOption<'_>> = units
        .iter()
        .enumerate()
        .map(|(index, (value, text))| InputSelectOption {
            value,
            label: text,
            selected: if matched {
                *value == current
            } else {
                index == 0
            },
        })
        .collect();
    input_select(InputSelect {
        label,
        name,
        required: true,
        options: &options,
        ..Default::default()
    })
}

#[html_form]
pub struct StockFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,
}

/// Lines editor on the movement form (Alpine + hidden JSON).
pub struct MovementLinesDraft;

impl FormWidget for MovementLinesDraft {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        input_movement_lines(InputMovementLines {
            label: field.label,
            name: field.name,
            defaults: field.value,
            ..Default::default()
        })
    }
}

/// Datetime field plus a button that fills the current local time.
pub struct DatetimeNow;

impl FormWidget for DatetimeNow {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        html! {
            div {
                (input_datetime(InputDatetime {
                    label: field.label,
                    name: field.name,
                    value: field.value,
                    required: field.required,
                    ..Default::default()
                }))
                button type="button" class="btn btn-outline btn-sm mb-2"
                    onclick="var w=this.previousElementSibling,t=w&&w.querySelector('[data-lariv-date-text]');if(!t)return;var d=new Date(),p=function(n){return String(n).padStart(2,'0')};t.value=p(d.getDate())+'/'+p(d.getMonth()+1)+'/'+d.getFullYear()+' '+p(d.getHours())+':'+p(d.getMinutes())+':'+p(d.getSeconds())" {
                    "Now"
                }
            }
        }
    }
}

#[html_form]
pub struct MovementForm {
    #[form(label = "Movement number", widget = Text, hint = "Leave blank to use the stock movement number format.")]
    pub number: String,

    #[form(label = "Date & time", required, widget = DatetimeNow)]
    pub datetime: String,

    #[form(label = "Type", required, widget = Select, choices = "movement_type")]
    pub movement_type: String,

    #[form(label = "Bill to an individual", widget = Checkbox, model = "billToIndividual")]
    pub bill_to_individual: String,

    #[form(
        label = "Contact",
        required,
        widget = ForeignKey,
        route = ContactFkSelectRouteTag,
        swap_key = "inventory-movement-contact",
        display = "customer_individual",
        placeholder = "Select contact…",
        show = "billToIndividual"
    )]
    pub customer_individual: i64,

    #[form(
        label = "Company",
        required,
        widget = ForeignKey,
        route = CompanyFkSelectRouteTag,
        swap_key = "inventory-movement-company",
        display = "customer_company",
        placeholder = "Select company…",
        show = "!billToIndividual"
    )]
    pub customer_company: i64,

    #[form(label = "Vehicle type", widget = Text)]
    pub vehicle_type: String,

    #[form(label = "Vehicle number", widget = Text)]
    pub vehicle_number: String,

    #[form(label = "E-way bill", widget = Text)]
    pub eway_bill: String,

    #[form(
        label = "Driver",
        widget = ForeignKey,
        route = ContactFkSelectRouteTag,
        swap_key = "inventory-movement-driver",
        display = "driver",
        placeholder = "Select driver…"
    )]
    pub driver_id: i64,

    #[form(label = "Lines", required, widget = MovementLinesDraft)]
    pub lines_json: String,
}

#[html_form]
pub struct InventoryPreferencesForm {
    #[form(
        label = "Stock movement number format",
        widget = Text,
        hint = "Tokens: {{YYYY}} {{YY}} {{FISCAL_CODE}} {{SEQ}} {{FISCAL_SEQ}} {{TYPE}}. Blank uses SM-{{YYYY}}-{{SEQ}}."
    )]
    pub movement_number_format: String,

    #[form(label = "Company name", widget = Text)]
    pub company_name: String,

    #[form(label = "Address", widget = Textarea, rows = 4)]
    pub company_address: String,

    #[form(label = "Phone number", widget = Text)]
    pub company_phone: String,

    #[form(label = "Email", widget = Text)]
    pub company_email: String,

    #[form(label = "GSTIN", widget = Text)]
    pub company_gstin: String,

    #[form(label = "Terms and condition", widget = Textarea, rows = 6)]
    pub terms_and_conditions: String,

    #[form(
        label = "Logo",
        widget = ForeignKey,
        route = VNodeFileSelectRouteTag,
        swap_key = "inventory-pref-logo",
        display = "logo_vnode",
        placeholder = "Select logo file…"
    )]
    pub logo_vnode_id: String,

    #[form(
        label = "Signature",
        widget = ForeignKey,
        route = VNodeFileSelectRouteTag,
        swap_key = "inventory-pref-signature",
        display = "signature_vnode",
        placeholder = "Select signature file…"
    )]
    pub signature_vnode_id: String,

    #[form(label = "Stock movement in template", widget = MovementPdfTemplate, rows = 16)]
    pub movement_in_template: String,

    #[form(label = "Stock movement out template", widget = MovementPdfTemplate, rows = 16)]
    pub movement_out_template: String,
}

/// Typst code editor for a stock-movement PDF template, with preview and restore actions under the field.
pub struct MovementPdfTemplate;

impl FormWidget for MovementPdfTemplate {
    fn render(_ctx: &FormCtx<'_>, field: &FieldRender<'_>) -> Markup {
        let Some(kind) = MovementTemplateKind::from_field(field.name) else {
            return code_editor_input(CodeEditorInput {
                label: field.label,
                name: field.name,
                value: field.value,
                language: "typst",
                rows: field.spec.rows.unwrap_or(16),
                ..Default::default()
            });
        };
        let rows = field.spec.rows.unwrap_or(16);
        html! {
            (label_hint(
                field.label,
                None,
                html! {
                    (code_editor_input(CodeEditorInput {
                        label: "",
                        name: field.name,
                        value: field.value,
                        id: kind.field_id,
                        language: "typst",
                        rows,
                        max_height: "24rem",
                        ..Default::default()
                    }))
                    textarea id=(kind.default_id) hidden readonly {
                        (kind.default_src)
                    }
                    div class="flex justify-end gap-2 mt-2" {
                        button type="button" class="btn btn-ghost btn-sm"
                            onclick=(use_default_template_onclick(kind.field_id, kind.default_id)) {
                            "Use default template"
                        }
                        button type="button" class="btn btn-outline btn-sm"
                            hx-post=(kind.preview_path)
                            hx-target=(HTMX_TARGET_BODY_MODAL)
                            hx-swap=(HTMX_SWAP_BODY_MODAL)
                            hx-include="closest form"
                            hx-push-url="false" {
                            "Preview"
                        }
                    }
                },
            ))
        }
    }
}

struct MovementTemplateKind {
    field_id: &'static str,
    default_id: &'static str,
    default_src: &'static str,
    preview_path: String,
}

impl MovementTemplateKind {
    fn from_field(name: &str) -> Option<Self> {
        match name {
            "MovementInTemplate" => Some(Self {
                field_id: "movement-in-template-field",
                default_id: "default-movement-in-template",
                default_src: DEFAULT_MOVEMENT_IN_TEMPLATE,
                preview_path: MovementPdfPreviewInRouteTag.path(),
            }),
            "MovementOutTemplate" => Some(Self {
                field_id: "movement-out-template-field",
                default_id: "default-movement-out-template",
                default_src: DEFAULT_MOVEMENT_OUT_TEMPLATE,
                preview_path: MovementPdfPreviewOutRouteTag.path(),
            }),
            _ => None,
        }
    }
}

fn use_default_template_onclick(field_id: &str, default_id: &str) -> String {
    format!(
        "if (confirm('This will overwrite the template in the field with the default example template. Continue?')) {{ \
         const ta = document.getElementById('{field_id}'); \
         const def = document.getElementById('{default_id}'); \
         if (!ta || !def) return; \
         ta.value = def.value; \
         const root = ta.closest('[data-code-editor-root]'); \
         if (root) {{ root.dispatchEvent(new CustomEvent('code-editor:set', {{ detail: {{ value: def.value }} }})); }} \
         else {{ ta.dispatchEvent(new Event('change', {{ bubbles: true }})); }} }}"
    )
}

#[html_form]
pub struct MovementFilterForm {
    #[form(label = "Type", widget = Select, choices = "movement_type")]
    pub movement_type: String,
}

#[cfg(test)]
mod tests {
    use super::{InventoryPreferencesForm, InventoryPreferencesFormField};
    use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};

    #[test]
    fn movement_templates_render_typst_editors_with_actions() {
        let ctx = FormCtx::form::<InventoryPreferencesForm>(CsrfToken::current())
            .value(InventoryPreferencesFormField::MovementInTemplate, "#in")
            .value(InventoryPreferencesFormField::MovementOutTemplate, "#out");
        let html = InventoryPreferencesForm::render_inputs(&ctx).into_string();

        assert_eq!(html.matches(r#"data-language="typst""#).count(), 2, "{html}");
        assert!(html.contains(r#"id="movement-in-template-field""#), "{html}");
        assert!(html.contains(r#"id="movement-out-template-field""#), "{html}");
        assert!(html.contains("#in"), "{html}");
        assert!(html.contains("#out"), "{html}");
        assert!(html.contains("Stock movement in"), "{html}");
        assert!(html.contains("Stock movement out (delivery challan)"), "{html}");
        assert_eq!(html.matches("Use default template").count(), 2, "{html}");
        assert!(
            html.contains("/dashboard/inventory/preferences/preview-in"),
            "{html}"
        );
        assert!(
            html.contains("/dashboard/inventory/preferences/preview-out"),
            "{html}"
        );

        let in_editor = html
            .find(r#"id="movement-in-template-field""#)
            .expect("in editor");
        let in_preview = html
            .find("/dashboard/inventory/preferences/preview-in")
            .expect("in preview");
        let out_editor = html
            .find(r#"id="movement-out-template-field""#)
            .expect("out editor");
        let out_preview = html
            .find("/dashboard/inventory/preferences/preview-out")
            .expect("out preview");
        assert!(in_editor < in_preview);
        assert!(in_preview < out_editor);
        assert!(out_editor < out_preview);
    }
}
