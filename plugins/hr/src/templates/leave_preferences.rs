use frunk::Generic;
use maud::Markup;

use lariv_core::components::{
    ButtonSubmit, FormOpts, ShellChrome, button_submit, form, form_hx_post_main,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use crate::crumbs::leave_preferences_crumbs;
use crate::forms::{LeaveCalcPreferencesForm, LeaveCalcPreferencesFormField};
use crate::routes::HrLeavePrefsPostRouteTag;
use crate::templates::{app_scaffold, hr_menu, scaffold_main, scaffold_pane};

fn schedule_token(value: &str) -> &'static str {
    if value.trim() == crate::logic::leave_calc::SCHEDULE_YEARLY {
        crate::logic::leave_calc::SCHEDULE_YEARLY
    } else {
        crate::logic::leave_calc::SCHEDULE_MONTHLY
    }
}

fn schedule_x_data(casual: &str, sick: &str, privilege: &str) -> String {
    format!(
        "{{ casualSchedule: '{}', sickSchedule: '{}', privilegeSchedule: '{}' }}",
        schedule_token(casual),
        schedule_token(sick),
        schedule_token(privilege)
    )
}

fn choice_pairs(choices: &[(&str, &str)]) -> Vec<(String, String)> {
    choices
        .iter()
        .map(|(key, label)| ((*key).to_string(), (*label).to_string()))
        .collect()
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
    pub privilege: LeaveTypePrefsView,
    pub privilege_consecutive_required: String,
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
                title: "Leave preferences",
                subtitle: "Credit leave from consecutive perfect attendance. A perfect day is a weekday with a closed punch, not a holiday and not an approved leave. Enter 0 for leave allocated to skip that leave type. Privilege leave also skips when consecutive days is 0.",
                form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                inputs: LeaveCalcPreferencesForm::render_inputs(
                    &FormCtx::form::<LeaveCalcPreferencesForm>(CsrfToken::current())
                        .x_data(&schedule_x_data(
                            &self.casual.schedule_kind,
                            &self.sick.schedule_kind,
                            &self.privilege.schedule_kind,
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
                            self.privilege.leave_allocated.as_str(),
                        )
                        .value(
                            LeaveCalcPreferencesFormField::PrivilegeScheduleKind,
                            self.privilege.schedule_kind.as_str(),
                        )
                        .value(
                            LeaveCalcPreferencesFormField::PrivilegeMonth,
                            self.privilege.month.as_str(),
                        )
                        .value(
                            LeaveCalcPreferencesFormField::PrivilegeDay,
                            self.privilege.day.as_str(),
                        ),
                ),
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
        scaffold_pane(
            hr_menu("leave-preferences"),
            leave_preferences_crumbs(),
            self.body(),
        )
    }

    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(leave_preferences_crumbs(), self.body())
    }
}

impl RenderTemplate for LeaveCalcPreferencesPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Leave preferences — Lariv",
            chrome,
            hr_menu("leave-preferences"),
            leave_preferences_crumbs(),
            self.body(),
        )
    }
}
