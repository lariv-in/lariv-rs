//! Leave-calculation fields on the unified HR preferences page.

use async_trait::async_trait;
use maud::{Markup, html};
use sea_orm::DatabaseConnection;

use lariv_core::html_form::{CsrfToken, FormCtx, FormFieldKey, HtmlForm, render_field_specs};
use lariv_plugin_hr::hr_preferences_patch::{
    HrPreferenceGroup, HrPreferencesAddon, HrPreferencesPost,
};

use crate::entities::leaves::leave_calc_preference::Model;
use crate::forms::{LeaveCalcPreferencesForm, LeaveCalcPreferencesFormField as Field};
use crate::logic::leave_calc::{self, DAY_LAST};

const SUBTITLE: &str = "Credit leave from consecutive perfect attendance. A perfect day is a weekday with a closed punch, not a holiday and not an approved leave. Casual and sick leave are credited on their schedule. Privilege leave is credited as soon as its consecutive days are reached. Enter 0 for leave allocated to skip that leave type.";

/// Registers the leave-calc preference group.
#[derive(Clone, Copy, Default)]
pub struct LeavePreferencesAddon;

struct TypeView {
    leave_allocated: String,
    schedule_kind: String,
    month: String,
    day: String,
}

fn view_from_model(row: &Model) -> TypeView {
    TypeView {
        leave_allocated: row.leave_allocated.to_string(),
        schedule_kind: row.schedule_kind.clone(),
        month: row.month.unwrap_or(1).to_string(),
        day: if row.day_spec.trim().eq_ignore_ascii_case(DAY_LAST) {
            DAY_LAST.to_string()
        } else {
            row.day_spec.clone()
        },
    }
}

fn schedule_token(value: &str) -> &'static str {
    if value.trim() == leave_calc::SCHEDULE_YEARLY {
        leave_calc::SCHEDULE_YEARLY
    } else {
        leave_calc::SCHEDULE_MONTHLY
    }
}

fn choice_pairs(choices: &[(&str, &str)]) -> Vec<(String, String)> {
    choices
        .iter()
        .map(|(key, label)| ((*key).to_string(), (*label).to_string()))
        .collect()
}

fn render_inputs(
    timezone: &str,
    casual: &TypeView,
    sick: &TypeView,
    privilege_allocated: &str,
    privilege_consecutive: &str,
) -> Markup {
    let schedule_kinds = choice_pairs(LeaveCalcPreferencesForm::schedule_kind_choices());
    let months = choice_pairs(LeaveCalcPreferencesForm::month_choices());
    let days = choice_pairs(LeaveCalcPreferencesForm::day_choices());
    let x_data = format!(
        "{{ casualSchedule: '{}', sickSchedule: '{}' }}",
        schedule_token(&casual.schedule_kind),
        schedule_token(&sick.schedule_kind)
    );
    let ctx = FormCtx::form::<LeaveCalcPreferencesForm>(CsrfToken::current())
        .x_data(&x_data)
        .choices(Field::CasualScheduleKind, &schedule_kinds)
        .choices(Field::CasualMonth, &months)
        .choices(Field::CasualDay, &days)
        .choices(Field::SickScheduleKind, &schedule_kinds)
        .choices(Field::SickMonth, &months)
        .choices(Field::SickDay, &days)
        .value(Field::Timezone, timezone)
        .value(Field::CasualLeaveAllocated, casual.leave_allocated.as_str())
        .value(Field::CasualScheduleKind, casual.schedule_kind.as_str())
        .value(Field::CasualMonth, casual.month.as_str())
        .value(Field::CasualDay, casual.day.as_str())
        .value(Field::SickLeaveAllocated, sick.leave_allocated.as_str())
        .value(Field::SickScheduleKind, sick.schedule_kind.as_str())
        .value(Field::SickMonth, sick.month.as_str())
        .value(Field::SickDay, sick.day.as_str())
        .value(Field::PrivilegeConsecutiveRequired, privilege_consecutive)
        .value(Field::PrivilegeLeaveAllocated, privilege_allocated);
    html! {
        p class="text-sm opacity-70 mb-2" { (SUBTITLE) }
        (render_field_specs(LeaveCalcPreferencesForm::field_specs(), &ctx))
    }
}

#[async_trait]
impl HrPreferencesAddon for LeavePreferencesAddon {
    fn id(&self) -> &'static str {
        "hr-leaves"
    }

    async fn render_groups(&self, db: &DatabaseConnection) -> Vec<HrPreferenceGroup> {
        let inputs = match leave_calc::load_preferences(db).await {
            Ok(prefs) => render_inputs(
                &prefs[0].timezone,
                &view_from_model(&prefs[0]),
                &view_from_model(&prefs[1]),
                &prefs[2].leave_allocated.to_string(),
                &prefs[2].consecutive_required.to_string(),
            ),
            Err(err) => html! { p class="text-error" { (err) } },
        };
        vec![HrPreferenceGroup {
            title: "Leaves",
            inputs,
        }]
    }

    async fn save_from_form(
        &self,
        db: &DatabaseConnection,
        post: &HrPreferencesPost,
    ) -> Result<(), String> {
        let form: LeaveCalcPreferencesForm = post.deserialize().map_err(|err| err.to_string())?;
        leave_calc::save_preferences(db, &form).await
    }
}
