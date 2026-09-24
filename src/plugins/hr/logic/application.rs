use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, TransactionTrait};

use crate::html_form::UploadedFile;
use crate::plugins::filesystem::{
    node::{self, NodeFile},
    state::FilesystemState,
};
use crate::plugins::forms::{
    entities::form_response, handlers::forms::find_form, logic::answers::parse_answers_json,
};

use super::{
    applicant::{
        ApplicantInput, create_applicant_for_user, parse_optional_age, parse_optional_gender,
        parse_optional_text,
    },
    email::send_portal_credentials_email,
    job_form::find_job_form,
    person::{PersonInput, normalized_person_input, validate_person_input},
    user::{create_hr_user_with_password, generate_random_password},
};
use crate::plugins::hr::roles;

const HR_RESUMES_DIR: &str = "HR Resumes";

pub struct ApplicationInput {
    pub name: String,
    pub email: String,
    pub mobile: String,
    pub age: String,
    pub gender: String,
    pub address: String,
    pub remarks: String,
    pub answers_json: String,
    pub resume: Option<UploadedFile>,
}

pub async fn submit_job_application(
    db: &DatabaseConnection,
    fs: &FilesystemState,
    job_form_id: i64,
    input: ApplicationInput,
) -> Result<(), String> {
    let job_form = find_job_form(db, job_form_id)
        .await
        .ok_or_else(|| "job posting not found".to_string())?;
    let form = find_form(db, job_form.form_id)
        .await
        .ok_or_else(|| "application form not found".to_string())?;

    let person = normalized_person_input(&PersonInput {
        name: input.name,
        mobile: input.mobile,
        email: input.email,
    });
    validate_person_input(&person)?;
    let age = parse_optional_age(&input.age)?;
    let gender = parse_optional_gender(&input.gender)?;
    let address = parse_optional_text(&input.address);
    let remarks = parse_optional_text(&input.remarks);

    let answers = parse_answers_json(&input.answers_json, &form.questions)?;
    let password = generate_random_password(16);

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let user_id = create_hr_user_with_password(&txn, &person, roles::APPLICANT, &password).await?;
    let now = Utc::now();
    let response = form_response::ActiveModel {
        id: Default::default(),
        form_id: Set(form.id),
        answers: Set(answers),
        submitted_at: Set(now),
        name: Set(Some(person.name.clone())),
        email: Set(Some(person.email.clone())),
    };
    let response = response.insert(&txn).await.map_err(|e| e.to_string())?;

    let resume_vnode_id = if let Some(file) = input.resume {
        Some(store_resume(fs, &person.name, file).await?)
    } else {
        None
    };

    create_applicant_for_user(
        &txn,
        user_id,
        ApplicantInput {
            person: person.clone(),
            form_response_id: Some(response.id),
            age,
            gender,
            resume_vnode_id,
            job_form_id: Some(job_form.id),
            remarks,
            address,
        },
    )
    .await?;
    txn.commit().await.map_err(|e| e.to_string())?;

    if let Err(e) = send_portal_credentials_email(db, &person.email, &person.name, &password).await
    {
        tracing::warn!(
            email = %person.email,
            error = %e,
            "job application created but credentials email failed"
        );
    }

    Ok(())
}

async fn store_resume(
    fs: &FilesystemState,
    applicant_name: &str,
    file: UploadedFile,
) -> Result<i64, String> {
    let parent_id = node::ensure_directory_path(
        &fs.db,
        fs.store.as_ref(),
        None,
        &[HR_RESUMES_DIR.to_string()],
    )
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "failed to create HR Resumes folder".to_string())?;
    let parent = node::get_by_id(&fs.db, parent_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "HR Resumes folder not found".to_string())?;

    let original = node::sanitize_node_name(file.filename());
    let stem = if original.is_empty() {
        "resume".to_string()
    } else {
        original
    };
    let prefix = node::sanitize_node_name(applicant_name);
    let mut name = if prefix.is_empty() {
        stem.clone()
    } else {
        format!("{prefix}-{stem}")
    };
    let mut n = 2u32;
    while node::find_child(&fs.db, Some(parent.id), &name, false)
        .await
        .map_err(|e| e.to_string())?
        .is_some()
    {
        name = if prefix.is_empty() {
            format!("{stem}-{n}")
        } else {
            format!("{prefix}-{stem}-{n}")
        };
        n += 1;
        if n > 1000 {
            return Err("could not store resume with a unique name".to_string());
        }
    }

    let vnode = node::create(
        &fs.db,
        fs.store.as_ref(),
        name,
        false,
        Some(NodeFile::Upload(file)),
        Some(&parent),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(vnode.id)
}
