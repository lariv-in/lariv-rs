//! Dashboard profile gate.
//!
//! An employee is a user who has an `hr_employees` row. Role names are not person
//! types. A row whose profile is still unfinished has to be completed.

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::plugins::hr::entities::{
    applicant::{self, Entity as ApplicantEntity},
    employee::{self, Entity as EmployeeEntity},
    ex_employee::{self, Entity as ExEmployeeEntity},
};
use crate::plugins::hr::logic::profile::employee_profile_complete;
use crate::plugins::hr::roles;
use crate::plugins::users::state::AuthContext;
use crate::web::opt_or_log;

/// Which HR table row the signed-in user still needs to fill in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissingHrProfile {
    Applicant,
    Probation,
    Employee,
    ExEmployee,
}

pub fn is_hr_role(role: &str) -> bool {
    roles::ALL.contains(&role)
}

/// `None` when no profile gate form is needed.
///
/// Employee status comes from an employee row, not from the user's role. A partial
/// row stays on the form until the remaining fields are filled in.
pub async fn missing_hr_profile(
    db: &DatabaseConnection,
    auth: &AuthContext,
) -> Option<MissingHrProfile> {
    if crate::plugins::users::roles::Superuser::matches(&auth.role) {
        return None;
    }
    if let Some(employee) = employee_for_user(db, auth.user.id).await {
        if !employee_profile_complete(&employee) {
            return Some(if employee.is_probationary {
                MissingHrProfile::Probation
            } else {
                MissingHrProfile::Employee
            });
        }
        return None;
    }
    match auth.role.as_str() {
        roles::Applicant::NAME => {
            if has_applicant(db, auth.user.id).await {
                None
            } else {
                Some(MissingHrProfile::Applicant)
            }
        }
        roles::ExEmployee::NAME => {
            if has_ex_employee(db, auth.user.id).await {
                None
            } else {
                Some(MissingHrProfile::ExEmployee)
            }
        }
        _ => None,
    }
}

pub async fn employee_for_user(db: &DatabaseConnection, user_id: i64) -> Option<employee::Model> {
    opt_or_log(
        EmployeeEntity::find()
            .filter(employee::Column::UserId.eq(user_id))
            .one(db)
            .await,
        "find employee by user",
    )
}

pub async fn has_applicant(db: &DatabaseConnection, user_id: i64) -> bool {
    opt_or_log(
        ApplicantEntity::find()
            .filter(applicant::Column::UserId.eq(user_id))
            .one(db)
            .await,
        "find applicant by user",
    )
    .is_some()
}

pub async fn has_ex_employee(db: &DatabaseConnection, user_id: i64) -> bool {
    opt_or_log(
        ExEmployeeEntity::find()
            .filter(ex_employee::Column::UserId.eq(user_id))
            .one(db)
            .await,
        "find ex-employee by user",
    )
    .is_some()
}
