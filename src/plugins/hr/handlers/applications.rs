use std::sync::Arc;

use axum::{
    extract::{Multipart, Path},
    response::{IntoResponse, Response},
};

use crate::{
    grapesjs::GrapesJsCapability,
    html_form::{CsrfToken, HtmlForm},
    http::Cap,
    plugins::{
        filesystem::state::FilesystemState,
        forms::{handlers::forms::find_form, logic::questions::questions_to_json},
        website::state::WebsiteState,
    },
};

use crate::plugins::hr::{
    forms::JobApplicationForm,
    gender::ApplicantGender,
    logic::{application::submit_job_application, job_form::find_job_form},
    public_page::render_themed_public_page_arc,
    templates::job_forms::{JobApplicationPage, JobApplicationSuccessPage, JobApplicationValues},
};

fn gender_choices() -> Vec<(String, String)> {
    ApplicantGender::choices()
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

fn values_from_submit(body: &<JobApplicationForm as HtmlForm>::Submit) -> JobApplicationValues {
    JobApplicationValues {
        name: body.name.clone(),
        email: body.email.clone(),
        mobile: body.mobile.clone(),
        age: body.age.clone(),
        gender: body.gender.clone(),
        address: body.address.clone(),
        remarks: body.remarks.clone(),
        answers_json: body.answers_json.clone(),
    }
}

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
        return (
            axum::http::StatusCode::NOT_FOUND,
            "Application form not found",
        )
            .into_response();
    };

    let page = JobApplicationPage {
        job_form_id: job_form.id,
        job_title: job_form.job_title.clone(),
        salary_range: job_form.salary_range.clone(),
        experience_required: job_form.experience_required.clone(),
        description: job_form.description.clone(),
        questions_json: questions_to_json(&form.questions),
        values: JobApplicationValues::default(),
        gender_choices: gender_choices(),
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
    Cap(fs): Cap<FilesystemState>,
    Cap(website): Cap<WebsiteState>,
    Cap(grapes): Cap<Arc<GrapesJsCapability>>,
    Path(id): Path<i64>,
    csrf: CsrfToken,
    multipart: Multipart,
) -> Response {
    let Some(job_form) = find_job_form(&hr.db, id).await else {
        return (axum::http::StatusCode::NOT_FOUND, "Job posting not found").into_response();
    };
    let Some(form) = find_form(&hr.db, job_form.form_id).await else {
        return (
            axum::http::StatusCode::NOT_FOUND,
            "Application form not found",
        )
            .into_response();
    };

    let parsed = match JobApplicationForm::from_multipart(multipart, &csrf).await {
        Ok(body) => body,
        Err(e) => {
            let page = JobApplicationPage {
                job_form_id: job_form.id,
                job_title: job_form.job_title.clone(),
                salary_range: job_form.salary_range.clone(),
                experience_required: job_form.experience_required.clone(),
                description: job_form.description.clone(),
                questions_json: questions_to_json(&form.questions),
                values: JobApplicationValues::default(),
                gender_choices: gender_choices(),
                error: e.to_string(),
                submitted: false,
            };
            return render_themed_public_page_arc(
                &website.db,
                website.store.as_ref(),
                &grapes,
                &job_form.job_title,
                page.render_body(),
            )
            .await;
        }
    };

    let values = values_from_submit(&parsed);
    match submit_job_application(
        &hr.db,
        &fs,
        id,
        crate::plugins::hr::logic::application::ApplicationInput {
            name: parsed.name,
            email: parsed.email,
            mobile: parsed.mobile,
            age: parsed.age,
            gender: parsed.gender,
            address: parsed.address,
            remarks: parsed.remarks,
            answers_json: parsed.answers_json,
            resume: parsed.resume,
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
                values,
                gender_choices: gender_choices(),
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
