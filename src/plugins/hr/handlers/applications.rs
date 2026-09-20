use std::sync::Arc;

use axum::{
    extract::{Path, Form},
    response::{IntoResponse, Response},
};

use crate::{
    grapesjs::GrapesJsCapability,
    http::Cap,
    plugins::{
        forms::{handlers::forms::find_form, logic::questions::questions_to_json},
        website::state::WebsiteState,
    },
};

use crate::plugins::hr::{
    forms::JobApplicationBody,
    logic::{application::submit_job_application, job_form::find_job_form},
    public_page::render_themed_public_page_arc,
    templates::job_forms::{JobApplicationPage, JobApplicationSuccessPage},
};

pub async fn apply_get(
    Cap(hr): Cap<crate::plugins::hr::state::HrState>,
    Cap(website): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    Path(id): Path<i64>,
) -> Response {
    let Some(job_form) = find_job_form(&hr.db, id).await else {
        return (axum::http::StatusCode::NOT_FOUND, "Job posting not found").into_response();
    };
    let Some(form) = find_form(&hr.db, job_form.form_id).await else {
        return (axum::http::StatusCode::NOT_FOUND, "Application form not found").into_response();
    };

    let page = JobApplicationPage {
        job_form_id: job_form.id,
        job_title: job_form.job_title.clone(),
        salary_range: job_form.salary_range.clone(),
        experience_required: job_form.experience_required.clone(),
        description: job_form.description.clone(),
        questions_json: questions_to_json(&form.questions),
        error: String::new(),
        submitted: false,
    };
    render_themed_public_page_arc(
        &website.db,
        website.store.as_ref(),
        &grapes,
        &job_form.job_title,
        page.render_body(),
    )
    .await
}

pub async fn apply_post(
    Cap(hr): Cap<crate::plugins::hr::state::HrState>,
    Cap(website): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    Path(id): Path<i64>,
    Form(body): Form<JobApplicationBody>,
) -> Response {
    let Some(job_form) = find_job_form(&hr.db, id).await else {
        return (axum::http::StatusCode::NOT_FOUND, "Job posting not found").into_response();
    };
    let Some(form) = find_form(&hr.db, job_form.form_id).await else {
        return (axum::http::StatusCode::NOT_FOUND, "Application form not found").into_response();
    };

    match submit_job_application(
        &hr.db,
        id,
        crate::plugins::hr::logic::application::ApplicationInput {
            name: body.name,
            email: body.email,
            mobile: body.mobile,
            answers_json: body.answers_json,
        },
    )
    .await
    {
        Ok(()) => {
            let page = JobApplicationSuccessPage {
                job_title: job_form.job_title.clone(),
            };
            render_themed_public_page_arc(
                &website.db,
                website.store.as_ref(),
                &grapes,
                "Application submitted",
                page.render_body(),
            )
            .await
        }
        Err(error) => {
            let page = JobApplicationPage {
                job_form_id: job_form.id,
                job_title: job_form.job_title.clone(),
                salary_range: job_form.salary_range.clone(),
                experience_required: job_form.experience_required.clone(),
                description: job_form.description.clone(),
                questions_json: questions_to_json(&form.questions),
                error,
                submitted: false,
            };
            render_themed_public_page_arc(
                &website.db,
                website.store.as_ref(),
                &grapes,
                &job_form.job_title,
                page.render_body(),
            )
            .await
        }
    }
}
