use axum::response::{IntoResponse, Response};

use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout};
use lariv_plugin_users::middleware::RequireAuth;

use crate::entities::leaves::leave_calc_preference::Model;
use crate::entities::leaves::leave_type::LeaveType;
use crate::forms::LeaveCalcPreferencesForm;
use crate::logic::leave_calc::{self, DAY_LAST};
use crate::routes::HrLeavePrefsGetRouteTag;
use crate::state::HrState;
use crate::templates::leave_preferences::{LeaveCalcPreferencesPage, LeaveTypePrefsView};

fn page_from_models(prefs: &[Model; 3], error: String) -> LeaveCalcPreferencesPage {
    LeaveCalcPreferencesPage {
        timezone: prefs[0].timezone.clone(),
        casual: view_from_model(&prefs[0]),
        sick: view_from_model(&prefs[1]),
        privilege: view_from_model(&prefs[2]),
        privilege_consecutive_required: prefs[2].consecutive_required.to_string(),
        error,
    }
}

fn view_from_model(row: &Model) -> LeaveTypePrefsView {
    LeaveTypePrefsView {
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

fn page_from_form(form: LeaveCalcPreferencesForm, error: String) -> LeaveCalcPreferencesPage {
    LeaveCalcPreferencesPage {
        timezone: form.timezone,
        casual: LeaveTypePrefsView {
            leave_allocated: form.casual_leave_allocated,
            schedule_kind: form.casual_schedule_kind,
            month: form.casual_month,
            day: form.casual_day,
        },
        sick: LeaveTypePrefsView {
            leave_allocated: form.sick_leave_allocated,
            schedule_kind: form.sick_schedule_kind,
            month: form.sick_month,
            day: form.sick_day,
        },
        privilege: LeaveTypePrefsView {
            leave_allocated: form.privilege_leave_allocated,
            schedule_kind: form.privilege_schedule_kind,
            month: form.privilege_month,
            day: form.privilege_day,
        },
        privilege_consecutive_required: form.privilege_consecutive_required,
        error,
    }
}

fn render_page(
    page: LeaveCalcPreferencesPage,
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    htmx: &Htmx,
) -> Response {
    let slot_ctx = SlotCtx::from_auth(ctx);
    html_built_page_or_app_layout(&page, htmx, chrome, &slot_ctx).into_response()
}

/// HTTP handler: leave calculation preferences.
pub async fn get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    match leave_calc::load_preferences(&state.db).await {
        Ok(prefs) => render_page(
            page_from_models(&prefs, String::new()),
            &chrome,
            &ctx,
            &htmx,
        ),
        Err(err) => {
            let empty = Model {
                id: 0,
                created_at: None,
                updated_at: None,
                leave_type: LeaveType::Casual,
                consecutive_required: 0,
                leave_allocated: 0,
                schedule_kind: leave_calc::SCHEDULE_MONTHLY.to_string(),
                month: Some(1),
                day_spec: "1".to_string(),
                timezone: lariv_core::datetime::DEFAULT_TIMEZONE.to_string(),
            };
            render_page(
                page_from_models(&[empty.clone(), empty.clone(), empty], err),
                &chrome,
                &ctx,
                &htmx,
            )
        }
    }
}

/// HTTP handler: save leave calculation preferences.
pub async fn post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<LeaveCalcPreferencesForm>,
) -> Response {
    match leave_calc::save_preferences(&state.db, &form).await {
        Ok(()) => htmx.redirect(&HrLeavePrefsGetRouteTag.url()),
        Err(err) => render_page(page_from_form(form, err), &chrome, &ctx, &htmx),
    }
}
