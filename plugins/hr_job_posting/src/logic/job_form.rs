use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, EntityTrait,
};

use crate::entities::job_form::{self, Entity as JobFormEntity};
use crate::questions::{JobPostingQuestions, persist_job_questions};

pub struct JobFormInput {
    pub job_title: String,
    pub salary_range: Option<String>,
    pub experience_required: Option<String>,
    pub description: String,
    pub questions: JobPostingQuestions,
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub fn validate_job_form_input(input: &JobFormInput) -> Result<(), String> {
    if input.job_title.trim().is_empty() {
        return Err("job title is required".to_string());
    }
    if input.description.trim().is_empty() {
        return Err("description is required".to_string());
    }
    Ok(())
}

pub async fn find_job_form(db: &DatabaseConnection, id: i64) -> Option<job_form::Model> {
    lariv_core::web::opt_or_log(
        JobFormEntity::find_by_id(id).one(db).await,
        "find job form by id",
    )
}

pub async fn create_job_form(
    db: &DatabaseConnection,
    input: JobFormInput,
) -> Result<job_form::Model, String> {
    validate_job_form_input(&input)?;
    let now = Utc::now();
    let model = job_form::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        job_title: Set(input.job_title.trim().to_string()),
        salary_range: Set(normalize_optional(input.salary_range)),
        experience_required: Set(normalize_optional(input.experience_required)),
        description: Set(input.description.trim().to_string()),
        questions: Default::default(),
    };
    let mut model = model.insert(db).await.map_err(|e| e.to_string())?;
    persist_job_questions(db, model.id, &input.questions).await?;
    model.questions = input.questions;
    Ok(model)
}

pub async fn update_job_form<C: ConnectionTrait>(
    db: &C,
    id: i64,
    input: JobFormInput,
) -> Result<job_form::Model, String> {
    validate_job_form_input(&input)?;
    let existing = JobFormEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "job form not found".to_string())?;
    let now = Utc::now();
    let mut am: job_form::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.job_title = Set(input.job_title.trim().to_string());
    am.salary_range = Set(normalize_optional(input.salary_range));
    am.experience_required = Set(normalize_optional(input.experience_required));
    am.description = Set(input.description.trim().to_string());
    let mut model = am.update(db).await.map_err(|e| e.to_string())?;
    persist_job_questions(db, model.id, &input.questions).await?;
    model.questions = input.questions;
    Ok(model)
}

pub async fn delete_job_form(db: &DatabaseConnection, id: i64) -> Result<(), String> {
    JobFormEntity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
