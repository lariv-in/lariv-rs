use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, TransactionTrait,
};

use crate::plugins::hr::entities::{
    employee,
    probation::{self, Entity as ProbationEntity},
};
use crate::plugins::hr::logic::person::{
    PersonInput, normalized_person_input, validate_person_input,
};
use crate::plugins::hr::scope::find_probation_scoped;
use crate::plugins::users::state::AuthContext;

pub async fn hire_employee(
    db: &DatabaseConnection,
    probation_id: i64,
    auth: &AuthContext,
) -> Result<i64, String> {
    let probation = find_probation_scoped(db, probation_id, auth)
        .await
        .ok_or_else(|| "probation not found".to_string())?;

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let employee_id = insert_employee_from_probation(&txn, &probation).await?;
    ProbationEntity::delete_by_id(probation.id)
        .exec(&txn)
        .await
        .map_err(|e| e.to_string())?;
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(employee_id)
}

async fn insert_employee_from_probation(
    db: &DatabaseTransaction,
    probation: &probation::Model,
) -> Result<i64, String> {
    let now = Utc::now();
    let row = employee::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(probation.name.clone()),
        mobile: Set(probation.mobile.clone()),
        email: Set(probation.email.clone()),
        hired_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.id)
}

pub async fn create_employee<C: ConnectionTrait>(
    db: &C,
    input: PersonInput,
) -> Result<employee::Model, String> {
    validate_person_input(&input)?;
    let input = normalized_person_input(&input);
    let now = Utc::now();
    let model = employee::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(input.name),
        mobile: Set(input.mobile),
        email: Set(input.email),
        hired_at: Set(now),
    };
    model.insert(db).await.map_err(|e| e.to_string())
}

pub async fn update_employee<C: ConnectionTrait>(
    db: &C,
    employee_id: i64,
    input: PersonInput,
) -> Result<employee::Model, String> {
    validate_person_input(&input)?;
    let input = normalized_person_input(&input);
    let existing = crate::plugins::hr::entities::employee::Entity::find_by_id(employee_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "employee not found".to_string())?;
    let now = Utc::now();
    let mut am: employee::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(input.name);
    am.mobile = Set(input.mobile);
    am.email = Set(input.email);
    am.update(db).await.map_err(|e| e.to_string())
}
