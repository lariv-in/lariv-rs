use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::entities::{
    employee::{self, Entity as EmployeeEntity},
    leaves::{
        approved_leave::{self, Entity as ApprovedLeaveEntity},
        leave_application::{self, Entity as LeaveApplicationEntity},
        leave_type::LeaveType,
        rejected_leave::{self, Entity as RejectedLeaveEntity},
    },
};
use lariv_plugin_users::entities::user::Entity as UserEntity;
use lariv_plugin_users::roles::Superuser;
use lariv_plugin_users::state::AuthContext;

pub const FILTER_PENDING: &str = "pending";
pub const FILTER_APPROVED: &str = "approved";
pub const FILTER_REJECTED: &str = "rejected";

pub const STATUS_PENDING: &str = "Pending";
pub const STATUS_APPROVED: &str = "Approved";
pub const STATUS_REJECTED: &str = "Rejected";

pub struct LeaveApplicationInput {
    pub applied_by_id: i64,
    pub date: NaiveDate,
    pub reason: String,
    pub leave_type: LeaveType,
}

pub struct ApproveLeaveInput {
    pub approved_by_id: i64,
    pub approved_at: DateTime<Utc>,
}

pub struct RejectLeaveInput {
    pub rejected_by_id: i64,
    pub rejected_at: DateTime<Utc>,
    pub reason: Option<String>,
}

pub fn status_label(approved: bool, rejected: bool) -> &'static str {
    if approved {
        STATUS_APPROVED
    } else if rejected {
        STATUS_REJECTED
    } else {
        STATUS_PENDING
    }
}

pub async fn approved_application_ids(db: &DatabaseConnection, ids: &[i64]) -> HashSet<i64> {
    rows_with_application_id(
        db,
        ids,
        ApprovedLeaveEntity::find()
            .filter(approved_leave::Column::LeaveApplicationId.is_in(ids.to_vec())),
    )
    .await
}

pub async fn rejected_application_ids(db: &DatabaseConnection, ids: &[i64]) -> HashSet<i64> {
    rows_with_application_id(
        db,
        ids,
        RejectedLeaveEntity::find()
            .filter(rejected_leave::Column::LeaveApplicationId.is_in(ids.to_vec())),
    )
    .await
}

async fn rows_with_application_id<E>(
    db: &DatabaseConnection,
    ids: &[i64],
    query: sea_orm::Select<E>,
) -> HashSet<i64>
where
    E: EntityTrait,
    E::Model: ApplicationId,
{
    if ids.is_empty() {
        return HashSet::new();
    }
    query
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|row| row.application_id())
        .collect()
}

trait ApplicationId {
    fn application_id(&self) -> i64;
}

impl ApplicationId for approved_leave::Model {
    fn application_id(&self) -> i64 {
        self.leave_application_id
    }
}

impl ApplicationId for rejected_leave::Model {
    fn application_id(&self) -> i64 {
        self.leave_application_id
    }
}

pub async fn find_approval(
    db: &DatabaseConnection,
    leave_application_id: i64,
) -> Option<approved_leave::Model> {
    lariv_core::web::opt_or_log(
        ApprovedLeaveEntity::find()
            .filter(approved_leave::Column::LeaveApplicationId.eq(leave_application_id))
            .one(db)
            .await,
        "find approved leave",
    )
}

pub async fn find_rejection(
    db: &DatabaseConnection,
    leave_application_id: i64,
) -> Option<rejected_leave::Model> {
    lariv_core::web::opt_or_log(
        RejectedLeaveEntity::find()
            .filter(rejected_leave::Column::LeaveApplicationId.eq(leave_application_id))
            .one(db)
            .await,
        "find rejected leave",
    )
}

pub async fn create_leave_application(
    db: &DatabaseConnection,
    input: LeaveApplicationInput,
) -> Result<leave_application::Model, String> {
    validate_application(db, &input).await?;
    let now = Utc::now();
    leave_application::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        applied_by_id: Set(input.applied_by_id),
        date: Set(input.date),
        reason: Set(input.reason.trim().to_string()),
        leave_type: Set(input.leave_type),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn update_leave_application(
    db: &DatabaseConnection,
    id: i64,
    input: LeaveApplicationInput,
) -> Result<leave_application::Model, String> {
    validate_application(db, &input).await?;
    let existing = LeaveApplicationEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "leave application not found".to_string())?;
    let now = Utc::now();
    let mut am: leave_application::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.applied_by_id = Set(input.applied_by_id);
    am.date = Set(input.date);
    am.reason = Set(input.reason.trim().to_string());
    am.leave_type = Set(input.leave_type);
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn delete_leave_application(db: &DatabaseConnection, id: i64) -> Result<(), String> {
    LeaveApplicationEntity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
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

pub fn ensure_leave_editor(actor: &AuthContext, applied_by_id: i64) -> Result<(), String> {
    if actor_may_edit(&actor.role, actor.user.id, applied_by_id) {
        Ok(())
    } else {
        Err("Only a superuser or the person who applied for this leave can edit it.".to_string())
    }
}

pub async fn ensure_leave_approver(
    db: &DatabaseConnection,
    actor: &AuthContext,
    applied_by_id: i64,
) -> Result<(), String> {
    let manager_id = applicant_manager_id(db, applied_by_id).await?;
    if actor_may_approve(&actor.role, actor.user.id, manager_id) {
        Ok(())
    } else {
        Err("Only a superuser or this employee's manager can approve this leave.".to_string())
    }
}

pub async fn approve_leave(
    db: &DatabaseConnection,
    leave_application_id: i64,
    actor: &AuthContext,
    input: ApproveLeaveInput,
) -> Result<approved_leave::Model, String> {
    let application = LeaveApplicationEntity::find_by_id(leave_application_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "leave application not found".to_string())?;
    ensure_leave_approver(db, actor, application.applied_by_id).await?;
    ensure_pending(db, leave_application_id).await?;
    if !user_exists(db, input.approved_by_id).await {
        return Err("approved by is required".to_string());
    }
    let now = Utc::now();
    approved_leave::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        leave_application_id: Set(leave_application_id),
        approved_by_id: Set(input.approved_by_id),
        approved_at: Set(input.approved_at),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn revoke_approval(
    db: &DatabaseConnection,
    leave_application_id: i64,
    actor: &AuthContext,
) -> Result<(), String> {
    let application = LeaveApplicationEntity::find_by_id(leave_application_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "leave application not found".to_string())?;
    ensure_leave_approver(db, actor, application.applied_by_id).await?;
    let approval = find_approval(db, leave_application_id)
        .await
        .ok_or_else(|| "this leave is not approved".to_string())?;
    ApprovedLeaveEntity::delete_by_id(approval.id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn ensure_leave_rejector(actor: &AuthContext) -> Result<(), String> {
    if Superuser::matches(&actor.role) {
        Ok(())
    } else {
        Err("Only a superuser can revoke this rejection.".to_string())
    }
}

pub async fn revoke_rejection(
    db: &DatabaseConnection,
    leave_application_id: i64,
    actor: &AuthContext,
) -> Result<(), String> {
    ensure_leave_rejector(actor)?;
    let exists = LeaveApplicationEntity::find_by_id(leave_application_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .is_some();
    if !exists {
        return Err("leave application not found".to_string());
    }
    let rejection = find_rejection(db, leave_application_id)
        .await
        .ok_or_else(|| "this leave is not rejected".to_string())?;
    RejectedLeaveEntity::delete_by_id(rejection.id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn reject_leave(
    db: &DatabaseConnection,
    leave_application_id: i64,
    input: RejectLeaveInput,
) -> Result<rejected_leave::Model, String> {
    ensure_pending(db, leave_application_id).await?;
    if !user_exists(db, input.rejected_by_id).await {
        return Err("rejected by is required".to_string());
    }
    let now = Utc::now();
    rejected_leave::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        leave_application_id: Set(leave_application_id),
        rejected_by_id: Set(input.rejected_by_id),
        rejected_at: Set(input.rejected_at),
        reason: Set(input.reason),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

async fn validate_application(
    db: &DatabaseConnection,
    input: &LeaveApplicationInput,
) -> Result<(), String> {
    if !user_exists(db, input.applied_by_id).await {
        return Err("applied by is required".to_string());
    }
    if input.reason.trim().is_empty() {
        return Err("reason is required".to_string());
    }
    Ok(())
}

async fn ensure_pending(db: &DatabaseConnection, leave_application_id: i64) -> Result<(), String> {
    let exists = LeaveApplicationEntity::find_by_id(leave_application_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .is_some();
    if !exists {
        return Err("leave application not found".to_string());
    }
    if find_approval(db, leave_application_id).await.is_some() {
        return Err("this leave is already approved".to_string());
    }
    if find_rejection(db, leave_application_id).await.is_some() {
        return Err("this leave is already rejected".to_string());
    }
    Ok(())
}

async fn user_exists(db: &DatabaseConnection, id: i64) -> bool {
    if id <= 0 {
        return false;
    }
    lariv_core::web::opt_or_log(UserEntity::find_by_id(id).one(db).await, "find user by id")
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::{STATUS_APPROVED, STATUS_PENDING, STATUS_REJECTED, status_label};

    #[test]
    fn only_superuser_or_the_employees_manager_may_approve() {
        use super::actor_may_approve;
        use lariv_plugin_users::roles::{Superuser, Unassigned};

        assert!(actor_may_approve(Superuser::NAME, 1, None));
        assert!(actor_may_approve(Superuser::NAME, 1, Some(9)));
        assert!(actor_may_approve(Unassigned::NAME, 4, Some(4)));
        assert!(!actor_may_approve(Unassigned::NAME, 4, Some(9)));
        assert!(!actor_may_approve(Unassigned::NAME, 4, None));
        assert!(!actor_may_approve(Unassigned::NAME, 0, Some(0)));
    }

    #[test]
    fn only_superuser_or_the_applicant_may_edit() {
        use super::actor_may_edit;
        use lariv_plugin_users::roles::{Superuser, Unassigned};

        assert!(actor_may_edit(Superuser::NAME, 1, 9));
        assert!(actor_may_edit(Unassigned::NAME, 4, 4));
        assert!(!actor_may_edit(Unassigned::NAME, 4, 9));
        assert!(!actor_may_edit(Unassigned::NAME, 0, 0));
    }

    #[test]
    fn status_prefers_approval_then_rejection() {
        assert_eq!(status_label(false, false), STATUS_PENDING);
        assert_eq!(status_label(true, false), STATUS_APPROVED);
        assert_eq!(status_label(false, true), STATUS_REJECTED);
        assert_eq!(status_label(true, true), STATUS_APPROVED);
    }
}
