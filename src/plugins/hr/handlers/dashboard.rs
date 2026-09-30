//! HTTP handlers for `/dashboard` route override — profile gate for HR roles.

use axum::{body::Bytes, http::HeaderMap};

use crate::{
    apps::AppsCapability,
    components::{SharedChromeFolder, SlotCtx},
    html_form::{UrlencodedFields, verify_form_csrf},
    http::Cap,
    plugins::{
        dashboard::handlers::apps,
        hr::{
            forms::{ApplicantForm, EmployeeForm, PersonForm},
            handlers::{
                applicants::{
                    applicant_input_from_form, form_values_from_form, person_input_from_form,
                },
                employees::{employee_values_from_form, employee_write_from_form},
            },
            logic::{
                applicant::create_applicant_for_user,
                dashboard::{
                    MissingHrProfile, has_applicant, has_confirmed_employee, has_ex_employee,
                    has_probation, missing_hr_profile,
                },
                employee::{create_employee_for_user, create_probationary_employee_for_user},
                ex_employee::create_ex_employee_for_user,
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

fn parse_form<T: serde::de::DeserializeOwned>(
    bytes: &Bytes,
    headers: &HeaderMap,
) -> Result<T, String> {
    let fields = UrlencodedFields::parse(bytes).map_err(|e| e.to_string())?;
    verify_form_csrf(headers, &fields).map_err(|e| e.to_string())?;
    fields.deserialize().map_err(|e| e.to_string())
}

/// `GET /dashboard` — launchpad for superusers/non-HR/complete profiles, otherwise HR profile gate form.
pub async fn dashboard_get(
    Cap(state): Cap<HrState>,
    Cap(catalog): Cap<AppsCapability>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> maud::Markup {
    let missing = missing_hr_profile(&state.db, &ctx).await;
    let Some(kind) = missing else {
        return apps(Cap(catalog), Cap(chrome), RequireAuth(ctx), htmx).await;
    };

    let slot_ctx = SlotCtx::from_auth(&ctx);
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
            let values = EmployeeFormValues {
                name: ctx.user.name.clone(),
                mobile: ctx.user.phone.to_string(),
                email: ctx.user.email.to_string(),
                ..Default::default()
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

/// `POST /dashboard` — create the missing HR profile row for the logged-in user.
pub async fn dashboard_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    headers: HeaderMap,
    htmx: Htmx,
    bytes: Bytes,
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
            let form: Result<ApplicantForm, String> = parse_form(&bytes, &headers);
            match form {
                Ok(form) => match applicant_input_from_form(&form, &ctx) {
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
                },
                Err(e) => {
                    let values = ApplicantFormValues {
                        name: ctx.user.name.clone(),
                        mobile: ctx.user.phone.to_string(),
                        email: ctx.user.email.to_string(),
                        ..Default::default()
                    };
                    let page = HrDashboardGatePage::for_applicant(values, e);
                    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                }
            }
        }
        MissingHrProfile::Probation => {
            let form: Result<EmployeeForm, String> = parse_form(&bytes, &headers);
            match form {
                Ok(form) => match employee_write_from_form(&state.db, &form).await {
                    Ok(input) => {
                        if has_probation(&state.db, ctx.user.id).await {
                            let page = HrDashboardSuccessPage::new();
                            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx);
                        }
                        match create_probationary_employee_for_user(&state.db, ctx.user.id, input)
                            .await
                        {
                            Ok(_) => {
                                let page = HrDashboardSuccessPage::new();
                                html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                            }
                            Err(e) => {
                                let values = employee_values_from_form(&state.db, &form).await;
                                let page = HrDashboardGatePage::for_employee(kind, values, e);
                                html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                            }
                        }
                    }
                    Err(e) => {
                        let values = employee_values_from_form(&state.db, &form).await;
                        let page = HrDashboardGatePage::for_employee(kind, values, e);
                        html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                    }
                },
                Err(e) => {
                    let values = EmployeeFormValues {
                        name: ctx.user.name.clone(),
                        mobile: ctx.user.phone.to_string(),
                        email: ctx.user.email.to_string(),
                        ..Default::default()
                    };
                    let page = HrDashboardGatePage::for_employee(kind, values, e);
                    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                }
            }
        }
        MissingHrProfile::Employee => {
            let form: Result<EmployeeForm, String> = parse_form(&bytes, &headers);
            match form {
                Ok(form) => match employee_write_from_form(&state.db, &form).await {
                    Ok(input) => {
                        if has_confirmed_employee(&state.db, ctx.user.id).await {
                            let page = HrDashboardSuccessPage::new();
                            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx);
                        }
                        match create_employee_for_user(&state.db, ctx.user.id, input).await {
                            Ok(_) => {
                                let page = HrDashboardSuccessPage::new();
                                html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                            }
                            Err(e) => {
                                let values = employee_values_from_form(&state.db, &form).await;
                                let page = HrDashboardGatePage::for_employee(kind, values, e);
                                html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                            }
                        }
                    }
                    Err(e) => {
                        let values = employee_values_from_form(&state.db, &form).await;
                        let page = HrDashboardGatePage::for_employee(kind, values, e);
                        html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                    }
                },
                Err(e) => {
                    let values = EmployeeFormValues {
                        name: ctx.user.name.clone(),
                        mobile: ctx.user.phone.to_string(),
                        email: ctx.user.email.to_string(),
                        ..Default::default()
                    };
                    let page = HrDashboardGatePage::for_employee(kind, values, e);
                    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                }
            }
        }
        MissingHrProfile::ExEmployee => {
            let form: Result<PersonForm, String> = parse_form(&bytes, &headers);
            match form {
                Ok(form) => {
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
                            let page = HrDashboardGatePage::for_ex_employee(
                                form.name,
                                form.mobile,
                                form.email,
                                e,
                            );
                            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                        }
                    }
                }
                Err(e) => {
                    let page = HrDashboardGatePage::for_ex_employee(
                        ctx.user.name.clone(),
                        ctx.user.phone.to_string(),
                        ctx.user.email.to_string(),
                        e,
                    );
                    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx)
                }
            }
        }
    }
}
