use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, TransactionTrait,
};

use crate::plugins::hr::entities::{
    applicant::{self, Entity as ApplicantEntity},
    employee,
};
use crate::plugins::hr::logic::person::{
    PersonInput, normalized_person_input, validate_person_input,
};
use crate::plugins::hr::logic::profile::{EmployeeProfile, apply_profile};
use crate::plugins::hr::logic::user::create_hr_user;
use crate::plugins::hr::scope::find_applicant_scoped;
use crate::plugins::users::roles::Unassigned;
use crate::plugins::users::state::AuthContext;

pub struct EmployeeWrite {
    pub person: PersonInput,
    pub profile: EmployeeProfile,
}

pub async fn hire_applicant(
    db: &DatabaseConnection,
    applicant_id: i64,
    auth: &AuthContext,
) -> Result<i64, String> {
    let applicant = find_applicant_scoped(db, applicant_id, auth)
        .await
        .ok_or_else(|| "applicant not found".to_string())?;

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let employee_id = insert_employee_from_applicant(&txn, &applicant).await?;
    ApplicantEntity::delete_by_id(applicant.id)
        .exec(&txn)
        .await
        .map_err(|e| e.to_string())?;
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(employee_id)
}

async fn insert_employee_from_applicant(
    db: &DatabaseTransaction,
    applicant: &applicant::Model,
) -> Result<i64, String> {
    let now = Utc::now();
    let row = employee::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        user_id: Set(applicant.user_id),
        name: Set(Some(applicant.name.clone())),
        mobile: Set(Some(applicant.mobile.clone())),
        email: Set(Some(applicant.email.clone())),
        hired_at: Set(now),
        is_probationary: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.id)
}

pub async fn create_employee(
    db: &DatabaseConnection,
    input: EmployeeWrite,
) -> Result<employee::Model, String> {
    validate_person_input(&input.person)?;
    let person = normalized_person_input(&input.person);
    let user_id = create_hr_user(db, &person, Unassigned::NAME).await?;
    insert_employee_for_user(
        db,
        user_id,
        EmployeeWrite {
            person,
            profile: input.profile,
        },
        false,
    )
    .await
}

pub async fn create_probationary_employee(
    db: &DatabaseConnection,
    input: EmployeeWrite,
) -> Result<employee::Model, String> {
    validate_person_input(&input.person)?;
    let person = normalized_person_input(&input.person);
    let user_id = create_hr_user(db, &person, Unassigned::NAME).await?;
    insert_employee_for_user(
        db,
        user_id,
        EmployeeWrite {
            person,
            profile: input.profile,
        },
        true,
    )
    .await
}

pub async fn create_employee_for_user(
    db: &DatabaseConnection,
    user_id: i64,
    input: EmployeeWrite,
) -> Result<employee::Model, String> {
    insert_employee_for_user(db, user_id, input, false).await
}

pub async fn create_probationary_employee_for_user(
    db: &DatabaseConnection,
    user_id: i64,
    input: EmployeeWrite,
) -> Result<employee::Model, String> {
    insert_employee_for_user(db, user_id, input, true).await
}

async fn insert_employee_for_user(
    db: &DatabaseConnection,
    user_id: i64,
    input: EmployeeWrite,
    is_probationary: bool,
) -> Result<employee::Model, String> {
    validate_person_input(&input.person)?;
    let person = normalized_person_input(&input.person);
    let now = Utc::now();
    let mut model = employee::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        user_id: Set(user_id),
        name: Set(Some(person.name)),
        mobile: Set(Some(person.mobile)),
        email: Set(Some(person.email)),
        hired_at: Set(now),
        is_probationary: Set(is_probationary),
        ..Default::default()
    };
    apply_profile(&mut model, &input.profile);
    model.insert(db).await.map_err(|e| e.to_string())
}

pub async fn update_employee<C: ConnectionTrait>(
    db: &C,
    employee_id: i64,
    input: EmployeeWrite,
) -> Result<employee::Model, String> {
    validate_person_input(&input.person)?;
    let person = normalized_person_input(&input.person);
    let existing = crate::plugins::hr::entities::employee::Entity::find_by_id(employee_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "employee not found".to_string())?;
    let now = Utc::now();
    let mut am: employee::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(Some(person.name));
    am.mobile = Set(Some(person.mobile));
    am.email = Set(Some(person.email));
    apply_profile(&mut am, &input.profile);
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn delete_employee<C: ConnectionTrait>(db: &C, employee_id: i64) -> Result<(), String> {
    employee::Entity::delete_by_id(employee_id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
