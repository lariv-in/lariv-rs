use axum::response::{IntoResponse, Response};

use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout};
use lariv_plugin_users::middleware::RequireAuth;

use crate::entities::leaves::leave_calc_preference::Model;
use crate::entities::leaves::leave_type::LeaveType;
use crate::forms::LeaveCalcPreferencesForm;
use crate::logic::employee_code::validate_employee_id_format;
use crate::logic::leave_calc::{self, DAY_LAST};
use crate::logic::preferences::{
    ensure_id_card_signing_key, load_employee_id_format, load_id_card_format,
    save_employee_id_format, save_id_card_format, save_id_card_signing_key,
    validate_id_card_signing_key,
};
use crate::routes::HrLeavePrefsGetRouteTag;
use crate::state::HrState;
use crate::templates::preferences::{LeaveCalcPreferencesPage, LeaveTypePrefsView};

fn page_from_models(
    prefs: &[Model; 3],
    employee_id_format: String,
    id_card_format: String,
    id_card_signing_key: String,
    error: String,
) -> LeaveCalcPreferencesPage {
    LeaveCalcPreferencesPage {
        timezone: prefs[0].timezone.clone(),
        casual: view_from_model(&prefs[0]),
        sick: view_from_model(&prefs[1]),
        privilege_leave_allocated: prefs[2].leave_allocated.to_string(),
        privilege_consecutive_required: prefs[2].consecutive_required.to_string(),
        employee_id_format,
        id_card_format,
        id_card_signing_key,
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
        privilege_leave_allocated: form.privilege_leave_allocated,
        privilege_consecutive_required: form.privilege_consecutive_required,
        employee_id_format: form.employee_id_format,
        id_card_format: form.id_card_format,
        id_card_signing_key: form.id_card_signing_key,
        error,
    }
}

fn preference_fields(
    employee_id_format: Result<String, String>,
    id_card: Result<String, String>,
    signing_key: Result<String, String>,
) -> (String, String, String, String) {
    let mut values = Vec::new();
    let mut errors = Vec::new();
    for result in [employee_id_format, id_card, signing_key] {
        match result {
            Ok(value) => values.push(value),
            Err(err) => {
                values.push(String::new());
                errors.push(err);
            }
        }
    }
    let employee_id_format = values.remove(0);
    let id_card_format = values.remove(0);
    let id_card_signing_key = values.remove(0);
    (
        employee_id_format,
        id_card_format,
        id_card_signing_key,
        errors.join("; "),
    )
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

/// HTTP handler: HR preferences.
pub async fn get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    let employee_id_format = load_employee_id_format(&state.db).await;
    let id_card = load_id_card_format(&state.db).await;
    let signing_key = ensure_id_card_signing_key(&state.db).await;
    match leave_calc::load_preferences(&state.db).await {
        Ok(prefs) => {
            let (employee_id_format, id_card_format, id_card_signing_key, error) =
                preference_fields(employee_id_format, id_card, signing_key);
            render_page(
                page_from_models(
                    &prefs,
                    employee_id_format,
                    id_card_format,
                    id_card_signing_key,
                    error,
                ),
                &chrome,
                &ctx,
                &htmx,
            )
        }
        Err(err) => {
            let (employee_id_format, id_card_format, id_card_signing_key, field_error) =
                preference_fields(employee_id_format, id_card, signing_key);
            let err = if field_error.is_empty() {
                err
            } else {
                format!("{err}; {field_error}")
            };
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
                page_from_models(
                    &[empty.clone(), empty.clone(), empty],
                    employee_id_format,
                    id_card_format,
                    id_card_signing_key,
                    err,
                ),
                &chrome,
                &ctx,
                &htmx,
            )
        }
    }
}

/// HTTP handler: save HR preferences.
pub async fn post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<LeaveCalcPreferencesForm>,
) -> Response {
    if let Err(err) = validate_employee_id_format(&form.employee_id_format) {
        return render_page(page_from_form(form, err), &chrome, &ctx, &htmx);
    }
    if let Err(err) = validate_id_card_signing_key(&form.id_card_signing_key) {
        return render_page(page_from_form(form, err), &chrome, &ctx, &htmx);
    }
    if let Err(err) = leave_calc::save_preferences(&state.db, &form).await {
        return render_page(page_from_form(form, err), &chrome, &ctx, &htmx);
    }
    if let Err(err) = save_employee_id_format(&state.db, &form.employee_id_format).await {
        return render_page(page_from_form(form, err), &chrome, &ctx, &htmx);
    }
    if let Err(err) = save_id_card_format(&state.db, &form.id_card_format).await {
        return render_page(page_from_form(form, err), &chrome, &ctx, &htmx);
    }
    match save_id_card_signing_key(&state.db, &form.id_card_signing_key).await {
        Ok(()) => htmx.redirect(&HrLeavePrefsGetRouteTag.url()),
        Err(err) => render_page(page_from_form(form, err), &chrome, &ctx, &htmx),
    }
}
