use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, EntityTrait,
};

use crate::plugins::hr::entities::applicant::{self, Entity as ApplicantEntity};
use crate::plugins::hr::logic::person::{
    PersonInput, normalized_person_input, validate_person_input,
};
use crate::plugins::hr::logic::user::create_hr_user;
use crate::plugins::hr::roles;

pub async fn create_applicant(
    db: &DatabaseConnection,
    input: PersonInput,
) -> Result<applicant::Model, String> {
    validate_person_input(&input)?;
    let input = normalized_person_input(&input);
    let user_id = create_hr_user(db, &input, roles::APPLICANT).await?;
    create_applicant_for_user(db, user_id, input).await
}

pub async fn create_applicant_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    input: PersonInput,
) -> Result<applicant::Model, String> {
    validate_person_input(&input)?;
    let input = normalized_person_input(&input);
    let now = Utc::now();
    let model = applicant::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        user_id: Set(user_id),
        name: Set(input.name),
        mobile: Set(input.mobile),
        email: Set(input.email),
    };
    model.insert(db).await.map_err(|e| e.to_string())
}

pub async fn update_applicant<C: ConnectionTrait>(
    db: &C,
    applicant_id: i64,
    input: PersonInput,
) -> Result<applicant::Model, String> {
    validate_person_input(&input)?;
    let input = normalized_person_input(&input);
    let existing = ApplicantEntity::find_by_id(applicant_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "applicant not found".to_string())?;
    let now = Utc::now();
    let mut am: applicant::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(input.name);
    am.mobile = Set(input.mobile);
    am.email = Set(input.email);
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn delete_applicant<C: ConnectionTrait>(db: &C, applicant_id: i64) -> Result<(), String> {
    ApplicantEntity::delete_by_id(applicant_id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
