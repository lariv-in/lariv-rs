//! Dashboard profile gate — detect missing HR rows for role-matched users.

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::plugins::hr::entities::{
    applicant::{self, Entity as ApplicantEntity},
    employee::{self, Entity as EmployeeEntity},
    ex_employee::{self, Entity as ExEmployeeEntity},
};
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

/// `None` when the dashboard apps launchpad should be shown.
pub async fn missing_hr_profile(
    db: &DatabaseConnection,
    auth: &AuthContext,
) -> Option<MissingHrProfile> {
    if auth.user.is_superuser {
        return None;
    }
    match auth.role.as_str() {
        roles::APPLICANT => {
            if has_applicant(db, auth.user.id).await {
                None
            } else {
                Some(MissingHrProfile::Applicant)
            }
        }
        roles::PROBATION => {
            if has_probation(db, auth.user.id).await {
                None
            } else {
                Some(MissingHrProfile::Probation)
            }
        }
        roles::EMPLOYEE => {
            if has_confirmed_employee(db, auth.user.id).await {
                None
            } else {
                Some(MissingHrProfile::Employee)
            }
        }
        roles::EX_EMPLOYEE => {
            if has_ex_employee(db, auth.user.id).await {
                None
            } else {
                Some(MissingHrProfile::ExEmployee)
            }
        }
        _ => None,
    }
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

pub async fn has_probation(db: &DatabaseConnection, user_id: i64) -> bool {
    opt_or_log(
        EmployeeEntity::find()
            .filter(employee::Column::UserId.eq(user_id))
            .filter(employee::Column::IsProbationary.eq(true))
            .one(db)
            .await,
        "find probation by user",
    )
    .is_some()
}

pub async fn has_confirmed_employee(db: &DatabaseConnection, user_id: i64) -> bool {
    opt_or_log(
        EmployeeEntity::find()
            .filter(employee::Column::UserId.eq(user_id))
            .filter(employee::Column::IsProbationary.eq(false))
            .one(db)
            .await,
        "find employee by user",
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
