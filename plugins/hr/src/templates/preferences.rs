use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_core::components::{
    ButtonSubmit, CodeEditorInput, FormOpts, ShellChrome,
    attrs::escape_attr,
    button_submit, code_editor_input, form, form_hx_post_main,
    htmx::{HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL},
};
use lariv_core::html_form::{CsrfToken, FormCtx, FormFieldKey, HtmlForm, render_field_specs};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use crate::crumbs::preferences_crumbs;
use crate::forms::{LeaveCalcPreferencesForm, LeaveCalcPreferencesFormField};
use crate::id_card_template::{DEFAULT_ID_CARD_TEMPLATE, ID_CARD_FORMAT_HINT};
use crate::routes::{HrIdCardPreviewPostRouteTag, HrLeavePrefsPostRouteTag};
use crate::templates::{app_scaffold, hr_menu, scaffold_main, scaffold_pane};

fn schedule_token(value: &str) -> &'static str {
    if value.trim() == crate::logic::leave_calc::SCHEDULE_YEARLY {
        crate::logic::leave_calc::SCHEDULE_YEARLY
    } else {
        crate::logic::leave_calc::SCHEDULE_MONTHLY
    }
}

fn schedule_x_data(casual: &str, sick: &str) -> String {
    format!(
        "{{ casualSchedule: '{}', sickSchedule: '{}' }}",
        schedule_token(casual),
        schedule_token(sick)
    )
}

fn choice_pairs(choices: &[(&str, &str)]) -> Vec<(String, String)> {
    choices
        .iter()
        .map(|(key, label)| ((*key).to_string(), (*label).to_string()))
        .collect()
}

fn id_card_format_field(value: &str) -> Markup {
    html! {
        (code_editor_input(CodeEditorInput {
            label: "ID card format",
            name: LeaveCalcPreferencesFormField::IdCardFormat.html_name(),
            value,
            id: "id-card-format-field",
            language: "plaintext",
            rows: 16,
            max_height: "24rem",
            hint: Some(ID_CARD_FORMAT_HINT),
            ..Default::default()
        }))
        textarea id="default-id-card-template" hidden readonly {
            (DEFAULT_ID_CARD_TEMPLATE)
        }
        div class="flex justify-end gap-2 mt-2" {
            button type="button" class="btn btn-ghost btn-sm"
                onclick="if (confirm('This will overwrite the template in the field with the default template. Continue?')) { const ta = document.getElementById('id-card-format-field'); const def = document.getElementById('default-id-card-template'); if (!ta || !def) return; ta.value = def.value; const root = ta.closest('[data-code-editor-root]'); if (root) { root.dispatchEvent(new CustomEvent('code-editor:set', { detail: { value: def.value } })); } else { ta.dispatchEvent(new Event('change', { bubbles: true })); } }" {
                "Use Default Template"
            }
            div class="fk-modal-host" {
                (PreEscaped(format!(
                    r#"<button type="button" class="btn btn-outline btn-sm" hx-post="{}" hx-target="{}" hx-swap="{}" hx-include="closest form" hx-push-url="false">Preview</button>"#,
                    HrIdCardPreviewPostRouteTag.path(),
                    escape_attr(HTMX_TARGET_BODY_MODAL),
                    escape_attr(HTMX_SWAP_BODY_MODAL),
                )))
            }
        }
    }
}

#[derive(Clone, Generic)]
pub struct LeaveTypePrefsView {
    pub leave_allocated: String,
    pub schedule_kind: String,
    pub month: String,
    pub day: String,
}

#[derive(Generic)]
pub struct LeaveCalcPreferencesPage {
    pub timezone: String,
    pub casual: LeaveTypePrefsView,
    pub sick: LeaveTypePrefsView,
    pub privilege_leave_allocated: String,
    pub privilege_consecutive_required: String,
    pub employee_id_format: String,
    pub id_card_format: String,
    pub id_card_signing_key: String,
    pub error: String,
}

impl LeaveCalcPreferencesPage {
    fn body(&self) -> Markup {
        let schedule_kinds = choice_pairs(LeaveCalcPreferencesForm::schedule_kind_choices());
        let months = choice_pairs(LeaveCalcPreferencesForm::month_choices());
        let days = choice_pairs(LeaveCalcPreferencesForm::day_choices());
        form(
            &CsrfToken::current(),
            FormOpts {
                attrs: form_hx_post_main(HrLeavePrefsPostRouteTag),
                title: "Preferences",
                subtitle: "Credit leave from consecutive perfect attendance. A perfect day is a weekday with a closed punch, not a holiday and not an approved leave. Casual and sick leave are credited on their schedule. Privilege leave is credited as soon as its consecutive days are reached. Enter 0 for leave allocated to skip that leave type.",
                form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                inputs: html! {
                    (render_field_specs(
                        &LeaveCalcPreferencesForm::field_specs()
                            .iter()
                            .copied()
                            .filter(|spec| {
                                spec.name != LeaveCalcPreferencesFormField::IdCardFormat.html_name()
                            })
                            .collect::<Vec<_>>(),
                        &FormCtx::form::<LeaveCalcPreferencesForm>(CsrfToken::current())
                            .x_data(&schedule_x_data(
                                &self.casual.schedule_kind,
                                &self.sick.schedule_kind,
                            ))
                            .choices(
                                LeaveCalcPreferencesFormField::CasualScheduleKind,
                                &schedule_kinds,
                            )
                            .choices(LeaveCalcPreferencesFormField::CasualMonth, &months)
                            .choices(LeaveCalcPreferencesFormField::CasualDay, &days)
                            .value(
                                LeaveCalcPreferencesFormField::Timezone,
                                self.timezone.as_str(),
                            )
                            .value(
                                LeaveCalcPreferencesFormField::CasualLeaveAllocated,
                                self.casual.leave_allocated.as_str(),
                            )
                            .value(
                                LeaveCalcPreferencesFormField::CasualScheduleKind,
                                self.casual.schedule_kind.as_str(),
                            )
                            .value(
                                LeaveCalcPreferencesFormField::CasualMonth,
                                self.casual.month.as_str(),
                            )
                            .value(
                                LeaveCalcPreferencesFormField::CasualDay,
                                self.casual.day.as_str(),
                            )
                            .value(
                                LeaveCalcPreferencesFormField::SickLeaveAllocated,
                                self.sick.leave_allocated.as_str(),
                            )
                            .value(
                                LeaveCalcPreferencesFormField::SickScheduleKind,
                                self.sick.schedule_kind.as_str(),
                            )
                            .value(
                                LeaveCalcPreferencesFormField::SickMonth,
                                self.sick.month.as_str(),
                            )
                            .value(
                                LeaveCalcPreferencesFormField::SickDay,
                                self.sick.day.as_str(),
                            )
                            .value(
                                LeaveCalcPreferencesFormField::PrivilegeConsecutiveRequired,
                                self.privilege_consecutive_required.as_str(),
                            )
                            .value(
                                LeaveCalcPreferencesFormField::PrivilegeLeaveAllocated,
                                self.privilege_leave_allocated.as_str(),
                            )
                            .value(
                                LeaveCalcPreferencesFormField::EmployeeIdFormat,
                                self.employee_id_format.as_str(),
                            )
                            .value(
                                LeaveCalcPreferencesFormField::IdCardSigningKey,
                                self.id_card_signing_key.as_str(),
                            ),
                    ))
                    (id_card_format_field(&self.id_card_format))
                },
                actions: button_submit(ButtonSubmit {
                    label: "Save preferences",
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
    }
}

impl RenderAppPane for LeaveCalcPreferencesPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(hr_menu("preferences"), preferences_crumbs(), self.body())
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(preferences_crumbs(), self.body())
    }
}

impl RenderTemplate for LeaveCalcPreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Preferences — Lariv",
            chrome,
            hr_menu("preferences"),
            preferences_crumbs(),
            self.body(),
        )
    }
}
