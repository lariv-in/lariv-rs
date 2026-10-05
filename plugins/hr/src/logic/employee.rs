use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, TransactionTrait,
};

use crate::entities::{
    applicant::{self, Entity as ApplicantEntity},
    employee,
};
use crate::logic::person::{
    PersonInput, normalized_person_input, validate_person_input,
};
use crate::logic::profile::{EmployeeProfile, apply_profile};
use crate::logic::user::{create_hr_user, user_is_superuser};
use crate::scope::find_applicant_scoped;
use lariv_plugin_users::roles::Unassigned;
use lariv_plugin_users::state::AuthContext;

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

/// Empty manager is allowed unless `employee_user_id` is a superuser.
pub async fn ensure_manager_for_superuser<C: ConnectionTrait>(
    db: &C,
    employee_user_id: i64,
    manager_id: Option<i64>,
) -> Result<(), String> {
    if positive_id(manager_id).is_some() {
        return Ok(());
    }
    if user_is_superuser(db, employee_user_id).await? {
        Err("Manager is required".to_string())
    } else {
        Ok(())
    }
}

fn positive_id(id: Option<i64>) -> Option<i64> {
    id.filter(|id| *id > 0)
}

async fn normalize_manager_id<C: ConnectionTrait>(
    db: &C,
    manager_id: Option<i64>,
) -> Result<Option<i64>, String> {
    let Some(id) = positive_id(manager_id) else {
        return Ok(None);
    };
    let exists = lariv_plugin_users::entities::user::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .is_some();
    if !exists {
        return Err("Choose a manager".to_string());
    }
    Ok(Some(id))
}

async fn insert_employee_for_user(
    db: &DatabaseConnection,
    user_id: i64,
    mut input: EmployeeWrite,
    is_probationary: bool,
) -> Result<employee::Model, String> {
    validate_person_input(&input.person)?;
    input.profile.manager_id = normalize_manager_id(db, input.profile.manager_id).await?;
    ensure_manager_for_superuser(db, user_id, input.profile.manager_id).await?;
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
    mut input: EmployeeWrite,
) -> Result<employee::Model, String> {
    validate_person_input(&input.person)?;
    input.profile.manager_id = normalize_manager_id(db, input.profile.manager_id).await?;
    let person = normalized_person_input(&input.person);
    let existing = crate::entities::employee::Entity::find_by_id(employee_id)
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
