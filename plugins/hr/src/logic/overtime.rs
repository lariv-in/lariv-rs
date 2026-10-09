use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter,
};

use crate::entities::{
    employee::{self, Entity as EmployeeEntity},
    overtime::{
        approved_overtime::{self, Entity as ApprovedOvertimeEntity},
        overtime_application::{self, Entity as OvertimeApplicationEntity},
        rejected_overtime::{self, Entity as RejectedOvertimeEntity},
    },
};
use lariv_plugin_users::entities::user::Entity as UserEntity;
use lariv_plugin_users::state::AuthContext;

pub use crate::logic::leave::{
    actor_may_approve, actor_may_edit, actor_may_view_leave, applicant_manager_id,
    managed_applicant_ids, user_is_leave_manager,
};

pub const FILTER_PENDING: &str = "pending";
pub const FILTER_APPROVED: &str = "approved";
pub const FILTER_REJECTED: &str = "rejected";

pub const STATUS_PENDING: &str = "Pending";
pub const STATUS_APPROVED: &str = "Approved";
pub const STATUS_REJECTED: &str = "Rejected";

pub struct OvertimeApplicationInput {
    pub user_id: i64,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub reason: Option<String>,
    pub timezone: String,
    /// Employees may only file overtime for today or yesterday.
    pub enforce_recent: bool,
}

pub struct ApprovedOvertimeInput {
    pub user_id: i64,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub approved_by_id: i64,
    pub approved_at: DateTime<Utc>,
}

pub struct ApproveOvertimeInput {
    pub approved_by_id: i64,
    pub approved_at: DateTime<Utc>,
}

pub struct RejectOvertimeInput {
    pub rejected_by_id: i64,
    pub rejected_at: DateTime<Utc>,
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

pub fn optional_reason(reason: &str) -> Option<String> {
    let trimmed = reason.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Both instants must fall on today or yesterday in `timezone`, and end after start.
pub fn interval_on_today_or_yesterday(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    timezone: &str,
    now: DateTime<Utc>,
) -> Result<(), String> {
    if end <= start {
        return Err("end must be after start".to_string());
    }
    let tz = lariv_core::datetime::parse_timezone(timezone);
    let today = now.with_timezone(&tz).date_naive();
    let start_date = start.with_timezone(&tz).date_naive();
    let end_date = end.with_timezone(&tz).date_naive();
    if !on_today_or_yesterday(start_date, today) || !on_today_or_yesterday(end_date, today) {
        return Err("overtime must be today or yesterday".to_string());
    }
    Ok(())
}

fn on_today_or_yesterday(date: NaiveDate, today: NaiveDate) -> bool {
    let yesterday = today.pred_opt().unwrap_or(today);
    date == today || date == yesterday
}

pub async fn approved_application_ids(db: &DatabaseConnection, ids: &[i64]) -> HashSet<i64> {
    rows_with_application_id(
        db,
        ids,
        ApprovedOvertimeEntity::find()
            .filter(approved_overtime::Column::OvertimeApplicationId.is_in(ids.to_vec())),
    )
    .await
}

pub async fn rejected_application_ids(db: &DatabaseConnection, ids: &[i64]) -> HashSet<i64> {
    rows_with_application_id(
        db,
        ids,
        RejectedOvertimeEntity::find()
            .filter(rejected_overtime::Column::OvertimeApplicationId.is_in(ids.to_vec())),
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
        .filter_map(|row| row.application_id())
        .collect()
}

trait ApplicationId {
    fn application_id(&self) -> Option<i64>;
}

impl ApplicationId for approved_overtime::Model {
    fn application_id(&self) -> Option<i64> {
        self.overtime_application_id.filter(|id| *id > 0)
    }
}

impl ApplicationId for rejected_overtime::Model {
    fn application_id(&self) -> Option<i64> {
        Some(self.overtime_application_id)
    }
}

pub async fn find_approval<C: ConnectionTrait>(
    db: &C,
    overtime_application_id: i64,
) -> Option<approved_overtime::Model> {
    lariv_core::web::opt_or_log(
        ApprovedOvertimeEntity::find()
            .filter(approved_overtime::Column::OvertimeApplicationId.eq(overtime_application_id))
            .one(db)
            .await,
        "find approved overtime",
    )
}

pub async fn find_rejection(
    db: &DatabaseConnection,
    overtime_application_id: i64,
) -> Option<rejected_overtime::Model> {
    lariv_core::web::opt_or_log(
        RejectedOvertimeEntity::find()
            .filter(rejected_overtime::Column::OvertimeApplicationId.eq(overtime_application_id))
            .one(db)
            .await,
        "find rejected overtime",
    )
}

pub fn ensure_overtime_editor(actor: &AuthContext, user_id: i64) -> Result<(), String> {
    if actor_may_edit(&actor.role, actor.user.id, user_id) {
        Ok(())
    } else {
        Err("Only a superuser or the person who applied for this overtime can edit it.".to_string())
    }
}

pub async fn ensure_overtime_approver(
    db: &DatabaseConnection,
    actor: &AuthContext,
    user_id: i64,
) -> Result<(), String> {
    let manager_id = applicant_manager_id(db, user_id).await?;
    if actor_may_approve(&actor.role, actor.user.id, manager_id) {
        Ok(())
    } else {
        Err("Only a superuser or this employee's manager can decide this overtime.".to_string())
    }
}

pub async fn create_overtime_application(
    db: &DatabaseConnection,
    input: OvertimeApplicationInput,
) -> Result<overtime_application::Model, String> {
    validate_application(db, &input).await?;
    let now = Utc::now();
    overtime_application::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        user_id: Set(input.user_id),
        start_time: Set(input.start_time),
        end_time: Set(input.end_time),
        reason: Set(input.reason),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn create_approved_overtime(
    db: &DatabaseConnection,
    input: ApprovedOvertimeInput,
) -> Result<approved_overtime::Model, String> {
    if !employee_exists(db, input.user_id).await {
        return Err("only an employee can have approved overtime".to_string());
    }
    if input.end_time <= input.start_time {
        return Err("end must be after start".to_string());
    }
    if !user_exists(db, input.approved_by_id).await {
        return Err("approved by is required".to_string());
    }
    let now = Utc::now();
    approved_overtime::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        user_id: Set(input.user_id),
        start_time: Set(input.start_time),
        end_time: Set(input.end_time),
        approved_by_id: Set(input.approved_by_id),
        approved_at: Set(input.approved_at),
        overtime_application_id: Set(None),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn update_overtime_application(
    db: &DatabaseConnection,
    id: i64,
    input: OvertimeApplicationInput,
) -> Result<overtime_application::Model, String> {
    validate_application(db, &input).await?;
    ensure_pending(db, id).await?;
    let existing = OvertimeApplicationEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "overtime application not found".to_string())?;
    let mut am: overtime_application::ActiveModel = existing.into();
    am.updated_at = Set(Some(Utc::now()));
    am.user_id = Set(input.user_id);
    am.start_time = Set(input.start_time);
    am.end_time = Set(input.end_time);
    am.reason = Set(input.reason);
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn delete_overtime_application(db: &DatabaseConnection, id: i64) -> Result<(), String> {
    ensure_pending(db, id).await?;
    OvertimeApplicationEntity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn approve_overtime(
    db: &DatabaseConnection,
    overtime_application_id: i64,
    actor: &AuthContext,
    input: ApproveOvertimeInput,
) -> Result<approved_overtime::Model, String> {
    let application = OvertimeApplicationEntity::find_by_id(overtime_application_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "overtime application not found".to_string())?;
    ensure_overtime_approver(db, actor, application.user_id).await?;
    ensure_pending(db, overtime_application_id).await?;
    if !user_exists(db, input.approved_by_id).await {
        return Err("approved by is required".to_string());
    }
    let now = Utc::now();
    approved_overtime::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        user_id: Set(application.user_id),
        start_time: Set(application.start_time),
        end_time: Set(application.end_time),
        approved_by_id: Set(input.approved_by_id),
        approved_at: Set(input.approved_at),
        overtime_application_id: Set(Some(overtime_application_id)),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn reject_overtime(
    db: &DatabaseConnection,
    overtime_application_id: i64,
    actor: &AuthContext,
    input: RejectOvertimeInput,
) -> Result<rejected_overtime::Model, String> {
    let application = OvertimeApplicationEntity::find_by_id(overtime_application_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "overtime application not found".to_string())?;
    ensure_overtime_approver(db, actor, application.user_id).await?;
    ensure_pending(db, overtime_application_id).await?;
    if !user_exists(db, input.rejected_by_id).await {
        return Err("rejected by is required".to_string());
    }
    let now = Utc::now();
    rejected_overtime::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        overtime_application_id: Set(overtime_application_id),
        rejected_by_id: Set(input.rejected_by_id),
        rejected_at: Set(input.rejected_at),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn revoke_approval(
    db: &DatabaseConnection,
    overtime_application_id: i64,
    actor: &AuthContext,
) -> Result<(), String> {
    let application = OvertimeApplicationEntity::find_by_id(overtime_application_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "overtime application not found".to_string())?;
    ensure_overtime_approver(db, actor, application.user_id).await?;
    let approval = find_approval(db, overtime_application_id)
        .await
        .ok_or_else(|| "this overtime is not approved".to_string())?;
    ApprovedOvertimeEntity::delete_by_id(approval.id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn revoke_rejection(
    db: &DatabaseConnection,
    overtime_application_id: i64,
    actor: &AuthContext,
) -> Result<(), String> {
    let application = OvertimeApplicationEntity::find_by_id(overtime_application_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "overtime application not found".to_string())?;
    ensure_overtime_approver(db, actor, application.user_id).await?;
    let rejection = find_rejection(db, overtime_application_id)
        .await
        .ok_or_else(|| "this overtime is not rejected".to_string())?;
    RejectedOvertimeEntity::delete_by_id(rejection.id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn validate_application(
    db: &DatabaseConnection,
    input: &OvertimeApplicationInput,
) -> Result<(), String> {
    if !user_exists(db, input.user_id).await {
        return Err("user is required".to_string());
    }
    if !employee_exists(db, input.user_id).await {
        return Err("only an employee can apply for overtime".to_string());
    }
    if input.end_time <= input.start_time {
        return Err("end must be after start".to_string());
    }
    if !input.enforce_recent {
        return Ok(());
    }
    interval_on_today_or_yesterday(
        input.start_time,
        input.end_time,
        &input.timezone,
        Utc::now(),
    )
}

async fn ensure_pending(
    db: &DatabaseConnection,
    overtime_application_id: i64,
) -> Result<(), String> {
    let exists = OvertimeApplicationEntity::find_by_id(overtime_application_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .is_some();
    if !exists {
        return Err("overtime application not found".to_string());
    }
    if find_approval(db, overtime_application_id).await.is_some() {
        return Err("this overtime is already approved".to_string());
    }
    if find_rejection(db, overtime_application_id).await.is_some() {
        return Err("this overtime is already rejected".to_string());
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

async fn employee_exists(db: &DatabaseConnection, user_id: i64) -> bool {
    if user_id <= 0 {
        return false;
    }
    lariv_core::web::opt_or_log(
        EmployeeEntity::find()
            .filter(employee::Column::UserId.eq(user_id))
            .one(db)
            .await,
        "find employee by user",
    )
    .is_some()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::{STATUS_APPROVED, STATUS_PENDING, STATUS_REJECTED};
    use super::{interval_on_today_or_yesterday, optional_reason, status_label};

    fn utc(y: i32, m: u32, d: u32, h: u32, min: u32) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    #[test]
    fn overtime_must_fall_on_today_or_yesterday() {
        let tz = "Asia/Kolkata";
        // 2026-10-09 12:00 IST.
        let now = utc(2026, 10, 9, 6, 30);
        assert!(interval_on_today_or_yesterday(
            utc(2026, 10, 9, 4, 0),
            utc(2026, 10, 9, 6, 0),
            tz,
            now,
        )
        .is_ok());
        // Yesterday evening into today.
        assert!(
            interval_on_today_or_yesterday(
                utc(2026, 10, 8, 17, 30),
                utc(2026, 10, 8, 19, 30),
                tz,
                now,
            )
            .is_ok()
        );
        // Yesterday, entirely.
        assert!(interval_on_today_or_yesterday(
            utc(2026, 10, 8, 4, 0),
            utc(2026, 10, 8, 6, 0),
            tz,
            now,
        )
        .is_ok());
        assert!(
            interval_on_today_or_yesterday(
                utc(2026, 10, 7, 4, 0),
                utc(2026, 10, 7, 6, 0),
                tz,
                now,
            )
            .is_err()
        );
        assert!(
            interval_on_today_or_yesterday(
                utc(2026, 10, 10, 4, 0),
                utc(2026, 10, 10, 6, 0),
                tz,
                now,
            )
            .is_err()
        );
        assert!(
            interval_on_today_or_yesterday(
                utc(2026, 10, 9, 6, 0),
                utc(2026, 10, 9, 4, 0),
                tz,
                now,
            )
            .is_err()
        );
    }

    #[test]
    fn only_superuser_or_the_employees_manager_may_decide() {
        use super::actor_may_approve;
        use lariv_plugin_users::roles::{Superuser, Unassigned};

        assert!(actor_may_approve(Superuser::NAME, 1, None));
        assert!(actor_may_approve(Superuser::NAME, 1, Some(9)));
        assert!(actor_may_approve(Unassigned::NAME, 4, Some(4)));
        assert!(!actor_may_approve(Unassigned::NAME, 4, Some(9)));
        assert!(!actor_may_approve(Unassigned::NAME, 4, None));
    }

    #[test]
    fn blank_reason_is_omitted() {
        assert_eq!(optional_reason("  "), None);
        assert_eq!(
            optional_reason(" late ship "),
            Some("late ship".to_string())
        );
    }

    #[test]
    fn status_prefers_approval_then_rejection() {
        assert_eq!(status_label(false, false), STATUS_PENDING);
        assert_eq!(status_label(true, false), STATUS_APPROVED);
        assert_eq!(status_label(false, true), STATUS_REJECTED);
    }
}
