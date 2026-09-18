use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, TransactionTrait,
};

use crate::plugins::hr::entities::{
    applicant::{self, Entity as ApplicantEntity},
    probation,
};
use crate::plugins::hr::logic::person::{normalized_person_input, validate_person_input, PersonInput};
use crate::plugins::hr::scope::find_applicant_scoped;
use crate::plugins::users::state::AuthContext;

pub async fn start_probation(
    db: &DatabaseConnection,
    applicant_id: i64,
    auth: &AuthContext,
) -> Result<i64, String> {
    let applicant = find_applicant_scoped(db, applicant_id, auth)
        .await
        .ok_or_else(|| "applicant not found".to_string())?;

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let probation_id = insert_probation_from_applicant(&txn, &applicant).await?;
    ApplicantEntity::delete_by_id(applicant.id)
        .exec(&txn)
        .await
        .map_err(|e| e.to_string())?;
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(probation_id)
}

async fn insert_probation_from_applicant(
    db: &DatabaseTransaction,
    applicant: &applicant::Model,
) -> Result<i64, String> {
    let now = Utc::now();
    let row = probation::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(applicant.name.clone()),
        mobile: Set(applicant.mobile.clone()),
        email: Set(applicant.email.clone()),
        started_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.id)
}

pub async fn create_probation<C: ConnectionTrait>(
    db: &C,
    input: PersonInput,
) -> Result<probation::Model, String> {
    validate_person_input(&input)?;
    let input = normalized_person_input(&input);
    let now = Utc::now();
    let model = probation::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(input.name),
        mobile: Set(input.mobile),
        email: Set(input.email),
        started_at: Set(now),
    };
    model.insert(db).await.map_err(|e| e.to_string())
}

pub async fn update_probation<C: ConnectionTrait>(
    db: &C,
    probation_id: i64,
    input: PersonInput,
) -> Result<probation::Model, String> {
    validate_person_input(&input)?;
    let input = normalized_person_input(&input);
    let existing = crate::plugins::hr::entities::probation::Entity::find_by_id(probation_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "probation not found".to_string())?;
    let now = Utc::now();
    let mut am: probation::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(input.name);
    am.mobile = Set(input.mobile);
    am.email = Set(input.email);
    am.update(db).await.map_err(|e| e.to_string())
}
