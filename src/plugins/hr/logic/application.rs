use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, TransactionTrait};

use crate::plugins::forms::{
    entities::form_response,
    handlers::forms::find_form,
    logic::answers::parse_answers_json,
};

use super::{
    applicant::create_applicant_for_user,
    email::send_portal_credentials_email,
    job_form::find_job_form,
    person::{PersonInput, normalized_person_input, validate_person_input},
    user::{create_hr_user_with_password, generate_random_password},
};
use crate::plugins::hr::roles;

pub struct ApplicationInput {
    pub name: String,
    pub email: String,
    pub mobile: String,
    pub answers_json: String,
}

pub async fn submit_job_application(
    db: &DatabaseConnection,
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

    let answers = parse_answers_json(&input.answers_json, &form.questions)?;
    let password = generate_random_password(16);

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let user_id =
        create_hr_user_with_password(&txn, &person, roles::APPLICANT, &password).await?;
    create_applicant_for_user(&txn, user_id, person.clone()).await?;
    let now = Utc::now();
    let response = form_response::ActiveModel {
        id: Default::default(),
        form_id: Set(form.id),
        answers: Set(answers),
        submitted_at: Set(now),
        name: Set(Some(person.name.clone())),
        email: Set(Some(person.email.clone())),
    };
    response.insert(&txn).await.map_err(|e| e.to_string())?;
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
