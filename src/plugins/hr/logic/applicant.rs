use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, EntityTrait,
};

use crate::plugins::hr::entities::applicant::{self, Entity as ApplicantEntity};
use crate::plugins::hr::gender::ApplicantGender;
use crate::plugins::hr::logic::person::{
    PersonInput, normalized_person_input, validate_person_input,
};
use crate::plugins::hr::logic::user::create_hr_user;
use crate::plugins::hr::roles;

#[derive(Clone)]
pub struct ApplicantInput {
    pub person: PersonInput,
    pub form_response_id: Option<i64>,
    pub age: Option<i64>,
    pub gender: Option<ApplicantGender>,
    pub resume_vnode_id: Option<i64>,
    pub job_form_id: Option<i64>,
    pub remarks: Option<String>,
    pub address: Option<String>,
}

pub fn parse_optional_fk(raw: &str) -> Option<i64> {
    raw.trim().parse().ok().filter(|id| *id > 0)
}

pub fn parse_optional_text(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn parse_optional_age(raw: &str) -> Result<Option<i64>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    crate::duration::parse_duration(trimmed).map(Some)
}

pub fn parse_optional_gender(raw: &str) -> Result<Option<ApplicantGender>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    ApplicantGender::parse(trimmed)
        .map(Some)
        .ok_or_else(|| format!("invalid gender: {trimmed:?}"))
}

pub async fn create_applicant(
    db: &DatabaseConnection,
    input: ApplicantInput,
) -> Result<applicant::Model, String> {
    validate_person_input(&input.person)?;
    let person = normalized_person_input(&input.person);
    let user_id = create_hr_user(db, &person, roles::APPLICANT).await?;
    create_applicant_for_user(db, user_id, ApplicantInput { person, ..input }).await
}

pub async fn create_applicant_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    input: ApplicantInput,
) -> Result<applicant::Model, String> {
    validate_person_input(&input.person)?;
    let person = normalized_person_input(&input.person);
    let now = Utc::now();
    let model = applicant::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        user_id: Set(user_id),
        name: Set(person.name),
        mobile: Set(person.mobile),
        email: Set(person.email),
        form_response_id: Set(input.form_response_id),
        age: Set(input.age),
        gender: Set(input.gender),
        resume_vnode_id: Set(input.resume_vnode_id),
        job_form_id: Set(input.job_form_id),
        remarks: Set(input.remarks),
        address: Set(input.address),
    };
    model.insert(db).await.map_err(|e| e.to_string())
}

pub async fn update_applicant<C: ConnectionTrait>(
    db: &C,
    applicant_id: i64,
    input: ApplicantInput,
) -> Result<applicant::Model, String> {
    validate_person_input(&input.person)?;
    let person = normalized_person_input(&input.person);
    let existing = ApplicantEntity::find_by_id(applicant_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "applicant not found".to_string())?;
    let now = Utc::now();
    let mut am: applicant::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(person.name);
    am.mobile = Set(person.mobile);
    am.email = Set(person.email);
    am.form_response_id = Set(input.form_response_id);
    am.age = Set(input.age);
    am.gender = Set(input.gender);
    am.resume_vnode_id = Set(input.resume_vnode_id);
    am.job_form_id = Set(input.job_form_id);
    am.remarks = Set(input.remarks);
    am.address = Set(input.address);
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn delete_applicant<C: ConnectionTrait>(db: &C, applicant_id: i64) -> Result<(), String> {
    ApplicantEntity::delete_by_id(applicant_id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
