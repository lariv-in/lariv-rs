use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, TransactionTrait,
};

use crate::entities::{
    employee::{self, Entity as EmployeeEntity},
    leaves::{
        approved_leave::{self, Entity as ApprovedLeaveEntity},
        leave_application::{self, Entity as LeaveApplicationEntity},
        leave_journal,
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

/// Posted when a leave becomes approved.
const JOURNAL_ON_APPROVE: i64 = -1;
/// Posted when a leave leaves the approved state.
const JOURNAL_ON_RELEASE: i64 = 1;

/// Casual, sick, then privilege. Sum of journal amounts for one user.
pub fn sum_leave_journal(entries: impl IntoIterator<Item = (LeaveType, i64)>) -> [i64; 3] {
    let mut totals = [0_i64; 3];
    for (leave_type, amount) in entries {
        let index = match leave_type {
            LeaveType::Casual => 0,
            LeaveType::Sick => 1,
            LeaveType::Privilege => 2,
        };
        totals[index] += amount;
    }
    totals
}

pub async fn leave_journal_balances(db: &DatabaseConnection, user_id: i64) -> [i64; 3] {
    let rows = leave_journal::Entity::find()
        .filter(leave_journal::Column::UserId.eq(user_id))
        .all(db)
        .await
        .unwrap_or_default();
    sum_leave_journal(rows.into_iter().map(|row| (row.leave_type, row.amount)))
}

pub fn format_journal_amount(amount: i64) -> String {
    amount.to_string()
}

/// Days credited by the give-leave form. Must be a positive whole number.
pub fn parse_give_leave_amount(raw: &str) -> Result<i64, String> {
    let amount = raw
        .trim()
        .parse::<i64>()
        .map_err(|_| "days must be a whole number".to_string())?;
    if amount <= 0 {
        return Err("days must be at least 1".to_string());
    }
    Ok(amount)
}

pub async fn give_leave(
    db: &DatabaseConnection,
    user_id: i64,
    leave_type: LeaveType,
    amount: i64,
) -> Result<(), String> {
    let amount = parse_give_leave_amount(&amount.to_string())?;
    if !user_exists(db, user_id).await {
        return Err("employee user is required".to_string());
    }
    append_leave_journal(db, user_id, leave_type, amount, Utc::now()).await
}

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

pub async fn find_approval<C: ConnectionTrait>(
    db: &C,
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
    let approved = find_approval(db, id).await.is_some();
    let user_or_type_changed =
        existing.applied_by_id != input.applied_by_id || existing.leave_type != input.leave_type;
    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now();
    if approved && user_or_type_changed {
        append_leave_journal(
            &txn,
            existing.applied_by_id,
            existing.leave_type,
            JOURNAL_ON_RELEASE,
            now,
        )
        .await?;
        append_leave_journal(
            &txn,
            input.applied_by_id,
            input.leave_type,
            JOURNAL_ON_APPROVE,
            now,
        )
        .await?;
    }
    let mut am: leave_application::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.applied_by_id = Set(input.applied_by_id);
    am.date = Set(input.date);
    am.reason = Set(input.reason.trim().to_string());
    am.leave_type = Set(input.leave_type);
    let row = am.update(&txn).await.map_err(|e| e.to_string())?;
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(row)
}

pub async fn delete_leave_application(db: &DatabaseConnection, id: i64) -> Result<(), String> {
    let application = LeaveApplicationEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    let txn = db.begin().await.map_err(|e| e.to_string())?;
    if let Some(application) = application {
        if find_approval(&txn, id).await.is_some() {
            append_leave_journal(
                &txn,
                application.applied_by_id,
                application.leave_type,
                JOURNAL_ON_RELEASE,
                Utc::now(),
            )
            .await?;
        }
    }
    LeaveApplicationEntity::delete_by_id(id)
        .exec(&txn)
        .await
        .map_err(|e| e.to_string())?;
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

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
    Superuser::matches(role)
        || (actor_user_id > 0 && actor_user_id == applied_by_id)
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
    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let row = approved_leave::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        leave_application_id: Set(leave_application_id),
        approved_by_id: Set(input.approved_by_id),
        approved_at: Set(input.approved_at),
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;
    append_leave_journal(
        &txn,
        application.applied_by_id,
        application.leave_type,
        JOURNAL_ON_APPROVE,
        input.approved_at,
    )
    .await?;
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(row)
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
    let txn = db.begin().await.map_err(|e| e.to_string())?;
    ApprovedLeaveEntity::delete_by_id(approval.id)
        .exec(&txn)
        .await
        .map_err(|e| e.to_string())?;
    append_leave_journal(
        &txn,
        application.applied_by_id,
        application.leave_type,
        JOURNAL_ON_RELEASE,
        Utc::now(),
    )
    .await?;
    txn.commit().await.map_err(|e| e.to_string())?;
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

async fn append_leave_journal<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    leave_type: LeaveType,
    amount: i64,
    at: DateTime<Utc>,
) -> Result<(), String> {
    let now = Utc::now();
    leave_journal::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        user_id: Set(user_id),
        datetime: Set(at),
        leave_type: Set(leave_type),
        amount: Set(amount),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;
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
    fn applicant_or_their_manager_may_view_a_leave() {
        use super::actor_may_view_leave;
        use lariv_plugin_users::roles::{Superuser, Unassigned};

        assert!(actor_may_view_leave(Superuser::NAME, 1, 9, None));
        assert!(actor_may_view_leave(Unassigned::NAME, 4, 4, Some(9)));
        assert!(actor_may_view_leave(Unassigned::NAME, 9, 4, Some(9)));
        assert!(!actor_may_view_leave(Unassigned::NAME, 3, 4, Some(9)));
        assert!(!actor_may_view_leave(Unassigned::NAME, 4, 9, None));
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

    #[test]
    fn journal_balance_sums_each_leave_type() {
        use super::sum_leave_journal;
        use crate::entities::leaves::LeaveType;

        let totals = sum_leave_journal([
            (LeaveType::Casual, -1),
            (LeaveType::Casual, 1),
            (LeaveType::Casual, -1),
            (LeaveType::Sick, -1),
            (LeaveType::Privilege, -1),
            (LeaveType::Privilege, -1),
        ]);
        assert_eq!(totals, [-1, -1, -2]);
    }

    #[test]
    fn give_leave_amount_must_be_a_positive_whole_number() {
        use super::parse_give_leave_amount;

        assert_eq!(parse_give_leave_amount("3"), Ok(3));
        assert!(parse_give_leave_amount("0").is_err());
        assert!(parse_give_leave_amount("-2").is_err());
        assert!(parse_give_leave_amount("1.5").is_err());
        assert!(parse_give_leave_amount("").is_err());
    }
}
