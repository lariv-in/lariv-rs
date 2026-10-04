//! HTTP handlers for `/dashboard` route override — profile gate.

use axum::extract::Multipart;

use crate::{
    apps::AppsCapability,
    components::{SharedChromeFolder, SlotCtx},
    html_form::{CsrfToken, HtmlForm},
    http::Cap,
    plugins::{
        dashboard::handlers::apps,
        filesystem::state::FilesystemState,
        hr::{
            forms::{ApplicantForm, EmployeeForm, PersonForm},
            handlers::{
                applicants::{
                    applicant_input_from_form, form_values_from_form, person_input_from_form,
                },
                employees::{
                    employee_values_from_model, employee_values_from_submit,
                    employee_write_from_submit,
                },
            },
            logic::{
                applicant::create_applicant_for_user,
                dashboard::{
                    MissingHrProfile, employee_for_user, has_applicant, has_ex_employee,
                    is_hr_role, missing_hr_profile,
                },
                employee::update_employee,
                ex_employee::create_ex_employee_for_user,
                profile::employee_profile_gaps,
            },
            state::HrState,
            templates::{
                ApplicantFormValues, EmployeeFormValues, HrDashboardGatePage,
                HrDashboardSuccessPage,
            },
        },
        users::middleware::RequireAuth,
    },
    web::{Htmx, html_built_page_or_app_layout},
};

/// `GET /dashboard` — profile gate for an unfinished employee form; submitted HR roles stay on
/// the success page (no apps launchpad yet); everyone else gets the apps launchpad.
pub async fn dashboard_get(
    Cap(state): Cap<HrState>,
    Cap(catalog): Cap<AppsCapability>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> maud::Markup {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let missing = missing_hr_profile(&state.db, &ctx).await;
    let Some(kind) = missing else {
        // HR self-service roles with a completed profile: hold on the success page.
        if is_hr_role(&ctx.role) && !crate::plugins::users::roles::Superuser::matches(&ctx.role) {
            let page = HrDashboardSuccessPage::new();
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx);
        }
        return apps(Cap(catalog), Cap(chrome), RequireAuth(ctx), htmx).await;
    };

    let page = match kind {
        MissingHrProfile::Applicant => {
            let values = ApplicantFormValues {
                name: ctx.user.name.clone(),
                mobile: ctx.user.phone.to_string(),
                email: ctx.user.email.to_string(),
                ..Default::default()
            };
            HrDashboardGatePage::for_applicant(values, String::new())
        }
        MissingHrProfile::Probation | MissingHrProfile::Employee => {
            let values = match employee_for_user(&state.db, ctx.user.id).await {
                Some(employee) => employee_values_from_model(&state.db, &employee).await,
                None => empty_employee_values(&ctx),
            };
            HrDashboardGatePage::for_employee(kind, values, String::new())
        }
        MissingHrProfile::ExEmployee => HrDashboardGatePage::for_ex_employee(
            ctx.user.name.clone(),
            ctx.user.phone.to_string(),
            ctx.user.email.to_string(),
            String::new(),
        ),
    };

    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
}

fn empty_employee_values(ctx: &crate::plugins::users::state::AuthContext) -> EmployeeFormValues {
    EmployeeFormValues {
        name: ctx.user.name.clone(),
        mobile: ctx.user.phone.to_string(),
        email: ctx.user.email.to_string(),
        ..Default::default()
    }
}

async fn handle_employee_gate_post(
    state: &HrState,
    fs: &FilesystemState,
    chrome: &SharedChromeFolder,
    ctx: &crate::plugins::users::state::AuthContext,
    htmx: &Htmx,
    slot_ctx: &SlotCtx,
    kind: MissingHrProfile,
    multipart: Multipart,
    csrf: &CsrfToken,
) -> maud::Markup {
    let Some(existing) = employee_for_user(&state.db, ctx.user.id).await else {
        let page = HrDashboardSuccessPage::new();
        return html_built_page_or_app_layout(&page, htmx, chrome, slot_ctx);
    };
    let saved = employee_values_from_model(&state.db, &existing).await;
    let submit = match EmployeeForm::from_multipart(multipart, csrf).await {
        Ok(submit) => submit,
        Err(e) => {
            let page = HrDashboardGatePage::for_employee(kind, saved, e.to_string());
            return html_built_page_or_app_layout(&page, htmx, chrome, slot_ctx);
        }
    };
    let values = employee_values_from_submit(&state.db, &submit, Some(&existing)).await;
    let input = match employee_write_from_submit(fs, submit, Some(&existing)).await {
        Ok(mut input) => {
            // These dates are not on the employee form, so a submit must not clear them.
            input.profile.date_of_joining = existing.date_of_joining;
            input.profile.probation_end_date = existing.probation_end_date;
            input
        }
        Err(e) => {
            let page = HrDashboardGatePage::for_employee(kind, values, e);
            return html_built_page_or_app_layout(&page, htmx, chrome, slot_ctx);
        }
    };

    match update_employee(&state.db, existing.id, input).await {
        Ok(updated) => {
            let gaps = employee_profile_gaps(&updated);
            if gaps.is_empty() {
                let page = HrDashboardSuccessPage::new();
                html_built_page_or_app_layout(&page, htmx, chrome, slot_ctx)
            } else {
                let values = employee_values_from_model(&state.db, &updated).await;
                let page = HrDashboardGatePage::for_employee(
                    kind,
                    values,
                    format!("Fill in the remaining fields: {}.", gaps.join(", ")),
                );
                html_built_page_or_app_layout(&page, htmx, chrome, slot_ctx)
            }
        }
        Err(e) => {
            let page = HrDashboardGatePage::for_employee(kind, values, e);
            html_built_page_or_app_layout(&page, htmx, chrome, slot_ctx)
        }
    }
}

/// `POST /dashboard` — create the missing HR profile row for the logged-in user.
pub async fn dashboard_post(
    Cap(state): Cap<HrState>,
    Cap(fs): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    csrf: CsrfToken,
    htmx: Htmx,
    multipart: Multipart,
) -> maud::Markup {
    let slot_ctx = SlotCtx::from_auth(&ctx);

    // Double-submit protection / gate check
    let missing = missing_hr_profile(&state.db, &ctx).await;
    let Some(kind) = missing else {
        let page = HrDashboardSuccessPage::new();
        return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx);
    };

    match kind {
        MissingHrProfile::Applicant => {
            let form = match ApplicantForm::from_multipart(multipart, &csrf).await {
                Ok(form) => form,
                Err(e) => {
                    let values = ApplicantFormValues {
                        name: ctx.user.name.clone(),
                        mobile: ctx.user.phone.to_string(),
                        email: ctx.user.email.to_string(),
                        ..Default::default()
                    };
                    let page = HrDashboardGatePage::for_applicant(values, e.to_string());
                    return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx);
                }
            };
            match applicant_input_from_form(&form, &ctx) {
                Ok(input) => {
                    if has_applicant(&state.db, ctx.user.id).await {
                        let page = HrDashboardSuccessPage::new();
                        return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx);
                    }
                    match create_applicant_for_user(&state.db, ctx.user.id, input).await {
                        Ok(_) => {
                            let page = HrDashboardSuccessPage::new();
                            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                        }
                        Err(e) => {
                            let values = form_values_from_form(&state.db, &form).await;
                            let page = HrDashboardGatePage::for_applicant(values, e);
                            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                        }
                    }
                }
                Err(e) => {
                    let values = form_values_from_form(&state.db, &form).await;
                    let page = HrDashboardGatePage::for_applicant(values, e);
                    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                }
            }
        }
        MissingHrProfile::Probation | MissingHrProfile::Employee => {
            handle_employee_gate_post(
                &state, &fs, &chrome, &ctx, &htmx, &slot_ctx, kind, multipart, &csrf,
            )
            .await
        }
        MissingHrProfile::ExEmployee => {
            let form = match PersonForm::from_multipart(multipart, &csrf).await {
                Ok(form) => form,
                Err(e) => {
                    let page = HrDashboardGatePage::for_ex_employee(
                        ctx.user.name.clone(),
                        ctx.user.phone.to_string(),
                        ctx.user.email.to_string(),
                        e.to_string(),
                    );
                    return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx);
                }
            };
            let input = person_input_from_form(&form);
            if has_ex_employee(&state.db, ctx.user.id).await {
                let page = HrDashboardSuccessPage::new();
                return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx);
            }
            match create_ex_employee_for_user(&state.db, ctx.user.id, input).await {
                Ok(_) => {
                    let page = HrDashboardSuccessPage::new();
                    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                }
                Err(e) => {
                    let page =
                        HrDashboardGatePage::for_ex_employee(form.name, form.mobile, form.email, e);
                    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                }
            }
        }
    }
}
