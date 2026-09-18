use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, DatabaseConnection, DatabaseTransaction, EntityTrait,
    TransactionTrait,
};

use crate::plugins::hr::entities::{
    employee::{self, Entity as EmployeeEntity},
    ex_employee,
};
use crate::plugins::hr::logic::person::{
    PersonInput, normalized_person_input, validate_person_input,
};
use crate::plugins::hr::scope::find_employee_scoped;
use crate::plugins::users::state::AuthContext;

pub async fn create_ex_employee<C: sea_orm::ConnectionTrait>(
    db: &C,
    input: PersonInput,
) -> Result<ex_employee::Model, String> {
    validate_person_input(&input)?;
    let input = normalized_person_input(&input);
    let now = Utc::now();
    let model = ex_employee::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        name: Set(input.name),
        mobile: Set(input.mobile),
        email: Set(input.email),
        terminated_at: Set(now),
    };
    model.insert(db).await.map_err(|e| e.to_string())
}

pub async fn terminate_employee(
    db: &DatabaseConnection,
    employee_id: i64,
    auth: &AuthContext,
) -> Result<i64, String> {
    let employee = find_employee_scoped(db, employee_id, auth)
        .await
        .ok_or_else(|| "employee not found".to_string())?;

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let ex_employee_id = insert_ex_employee_from_employee(&txn, &employee).await?;
    EmployeeEntity::delete_by_id(employee.id)
        .exec(&txn)
        .await
        .map_err(|e| e.to_string())?;
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(ex_employee_id)
}

async fn insert_ex_employee_from_employee(
    db: &DatabaseTransaction,
    employee: &employee::Model,
) -> Result<i64, String> {
    let now = Utc::now();
    let row = ex_employee::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        name: Set(employee.name.clone()),
        mobile: Set(employee.mobile.clone()),
        email: Set(employee.email.clone()),
        terminated_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.id)
}
