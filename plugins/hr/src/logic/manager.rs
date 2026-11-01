//! Employee-manager relationship helpers shared by leave, overtime and other approval flows.

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::entities::employee::{self, Entity as EmployeeEntity};
use lariv_plugin_users::roles::Superuser;

/// User ids of employees whose `manager_id` is `manager_user_id`.
pub async fn managed_applicant_ids(
    db: &DatabaseConnection,
    manager_user_id: i64,
) -> Result<Vec<i64>, String> {
    if manager_user_id <= 0 {
        return Ok(Vec::new());
    }
    let rows = EmployeeEntity::find()
        .filter(employee::Column::ManagerId.eq(manager_user_id))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|row| row.user_id)
        .filter(|id| *id > 0)
        .collect())
}

pub async fn user_is_leave_manager(db: &DatabaseConnection, user_id: i64) -> bool {
    managed_applicant_ids(db, user_id)
        .await
        .map(|ids| !ids.is_empty())
        .unwrap_or(false)
}

/// Superuser, the applicant, or that employee's manager.
pub fn actor_may_view_leave(
    role: &str,
    actor_user_id: i64,
    applied_by_id: i64,
    manager_id: Option<i64>,
) -> bool {
    actor_may_view_managed(role, actor_user_id, applied_by_id, manager_id)
}

/// Superuser, the record's owner, or the owner's manager.
pub fn actor_may_view_managed(
    role: &str,
    actor_user_id: i64,
    owner_user_id: i64,
    manager_id: Option<i64>,
) -> bool {
    Superuser::matches(role)
        || (actor_user_id > 0 && actor_user_id == owner_user_id)
        || actor_may_approve(role, actor_user_id, manager_id)
}

/// Superuser, or the user stored as this employee's manager.
pub fn actor_may_approve(role: &str, actor_user_id: i64, manager_id: Option<i64>) -> bool {
    if Superuser::matches(role) {
        return true;
    }
    actor_user_id > 0 && manager_id.filter(|id| *id > 0) == Some(actor_user_id)
}

pub async fn applicant_manager_id(
    db: &DatabaseConnection,
    applied_by_id: i64,
) -> Result<Option<i64>, String> {
    let employee = EmployeeEntity::find()
        .filter(employee::Column::UserId.eq(applied_by_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(employee.and_then(|row| row.manager_id.filter(|id| *id > 0)))
}

/// Superuser, or the user who submitted the application.
pub fn actor_may_edit(role: &str, actor_user_id: i64, applied_by_id: i64) -> bool {
    if Superuser::matches(role) {
        return true;
    }
    actor_user_id > 0 && actor_user_id == applied_by_id
}

#[cfg(test)]
mod tests {
    use super::{actor_may_approve, actor_may_edit, actor_may_view_leave, actor_may_view_managed};
    use lariv_plugin_users::roles::{Superuser, Unassigned};

    #[test]
    fn only_superuser_or_the_employees_manager_may_approve() {
        assert!(actor_may_approve(Superuser::NAME, 1, None));
        assert!(actor_may_approve(Superuser::NAME, 1, Some(9)));
        assert!(actor_may_approve(Unassigned::NAME, 4, Some(4)));
        assert!(!actor_may_approve(Unassigned::NAME, 4, Some(9)));
        assert!(!actor_may_approve(Unassigned::NAME, 4, None));
        assert!(!actor_may_approve(Unassigned::NAME, 0, Some(0)));
    }

    #[test]
    fn applicant_or_their_manager_may_view() {
        assert!(actor_may_view_leave(Superuser::NAME, 1, 9, None));
        assert!(actor_may_view_leave(Unassigned::NAME, 4, 4, Some(9)));
        assert!(actor_may_view_leave(Unassigned::NAME, 9, 4, Some(9)));
        assert!(!actor_may_view_leave(Unassigned::NAME, 3, 4, Some(9)));
        assert!(!actor_may_view_leave(Unassigned::NAME, 4, 9, None));
        assert!(actor_may_view_managed(Unassigned::NAME, 9, 4, Some(9)));
    }

    #[test]
    fn only_superuser_or_the_applicant_may_edit() {
        assert!(actor_may_edit(Superuser::NAME, 1, 9));
        assert!(actor_may_edit(Unassigned::NAME, 4, 4));
        assert!(!actor_may_edit(Unassigned::NAME, 4, 9));
        assert!(!actor_may_edit(Unassigned::NAME, 0, 0));
    }
}
