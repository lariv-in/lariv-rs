use std::collections::HashMap;

use chrono::{DateTime, NaiveTime, Timelike, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};

use crate::entities::attendance::{self, Entity as AttendanceEntity};
use lariv_plugin_users::entities::user::{self, Entity as UserEntity};

pub struct AttendanceInput {
    pub user_id: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
}

pub async fn user_display_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    lariv_core::web::opt_or_log(UserEntity::find_by_id(id).one(db).await, "find user by id")
        .map(|user| user.name)
        .unwrap_or_else(|| format!("User {id}"))
}

pub async fn user_display_labels(db: &DatabaseConnection, ids: &[i64]) -> HashMap<i64, String> {
    let ids: Vec<i64> = ids.iter().copied().filter(|id| *id > 0).collect();
    if ids.is_empty() {
        return HashMap::new();
    }
    let users = UserEntity::find()
        .filter(user::Column::Id.is_in(ids))
        .all(db)
        .await
        .unwrap_or_default();
    users.into_iter().map(|user| (user.id, user.name)).collect()
}

async fn user_exists(db: &DatabaseConnection, id: i64) -> bool {
    if id <= 0 {
        return false;
    }
    lariv_core::web::opt_or_log(UserEntity::find_by_id(id).one(db).await, "find user by id")
        .is_some()
}

async fn validate_attendance_input(
    db: &DatabaseConnection,
    input: &AttendanceInput,
) -> Result<(), String> {
    if !user_exists(db, input.user_id).await {
        return Err("user is required".to_string());
    }
    if input.ended_at <= input.started_at {
        return Err("end must be after start".to_string());
    }
    Ok(())
}

pub async fn create_attendance(
    db: &DatabaseConnection,
    input: AttendanceInput,
) -> Result<attendance::Model, String> {
    validate_attendance_input(db, &input).await?;
    let now = Utc::now();
    attendance::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        user_id: Set(input.user_id),
        started_at: Set(input.started_at),
        ended_at: Set(Some(input.ended_at)),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn update_attendance(
    db: &DatabaseConnection,
    id: i64,
    input: AttendanceInput,
) -> Result<attendance::Model, String> {
    validate_attendance_input(db, &input).await?;
    let existing = AttendanceEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "attendance not found".to_string())?;
    let now = Utc::now();
    let mut am: attendance::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.user_id = Set(input.user_id);
    am.started_at = Set(input.started_at);
    am.ended_at = Set(Some(input.ended_at));
    am.update(db).await.map_err(|e| e.to_string())
}

async fn require_employee(db: &DatabaseConnection, user_id: i64) -> Result<(), String> {
    if crate::logic::dashboard::employee_for_user(db, user_id)
        .await
        .is_none()
    {
        return Err("employee is required".to_string());
    }
    Ok(())
}

async fn open_attendance(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<Option<attendance::Model>, String> {
    AttendanceEntity::find()
        .filter(attendance::Column::UserId.eq(user_id))
        .filter(attendance::Column::EndedAt.is_null())
        .order_by_desc(attendance::Column::StartedAt)
        .order_by_desc(attendance::Column::Id)
        .one(db)
        .await
        .map_err(|e| e.to_string())
}

/// Whether the punch buttons can be used for this user right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PunchAvailability {
    pub punch_in_enabled: bool,
    pub punch_out_enabled: bool,
    pub note: String,
}

/// Employee shift and open attendance used to decide punch buttons.
pub struct PunchSubject {
    pub is_employee: bool,
    pub work_start: Option<NaiveTime>,
    pub already_in: bool,
    pub timezone: String,
}

const DAY_SECONDS: i64 = 24 * 60 * 60;

/// Minutes on either side of work start when preferences have not been saved.
pub const DEFAULT_PUNCH_IN_WINDOW_MINUTES: u32 = 15;

/// Punch-in window from HR preferences. Missing or unreadable rows use the default.
pub async fn load_punch_in_window_minutes(db: &DatabaseConnection) -> u32 {
    let minutes = lariv_core::web::opt_or_log(
        crate::entities::leaves::leave_calc_preference::Entity::find()
            .one(db)
            .await,
        "load punch in window",
    )
    .map(|row| row.punch_in_window_minutes)
    .unwrap_or(i64::from(DEFAULT_PUNCH_IN_WINDOW_MINUTES));
    u32::try_from(minutes.max(0)).unwrap_or(u32::MAX)
}

/// Signed seconds from `start` to `now` on a 24-hour clock, in `-12h..12h`.
fn signed_seconds_from(now: NaiveTime, start: NaiveTime) -> i64 {
    let mut delta =
        now.num_seconds_from_midnight() as i64 - start.num_seconds_from_midnight() as i64;
    if delta > DAY_SECONDS / 2 {
        delta -= DAY_SECONDS;
    } else if delta < -DAY_SECONDS / 2 {
        delta += DAY_SECONDS;
    }
    delta
}

fn window_label(minutes: u32) -> String {
    if minutes == 1 {
        "1 minute".to_string()
    } else {
        format!("{minutes} minutes")
    }
}

fn clock_label(time: NaiveTime) -> String {
    time.format("%H:%M").to_string()
}

/// Punch in is enabled when local time is within `window_minutes` of `work_start`.
///
/// The comparison uses the closest distance on the clock, so a shift that starts
/// just after midnight still opens late the previous evening.
pub fn punch_availability(
    now: DateTime<Utc>,
    timezone: &str,
    work_start: Option<NaiveTime>,
    already_in: bool,
    window_minutes: u32,
) -> PunchAvailability {
    if already_in {
        return PunchAvailability {
            punch_in_enabled: false,
            punch_out_enabled: true,
            note: "Already punched in.".to_string(),
        };
    }
    let Some(work_start) = work_start else {
        return PunchAvailability {
            punch_in_enabled: false,
            punch_out_enabled: false,
            note: "Set a work start time to punch in.".to_string(),
        };
    };
    let local = now
        .with_timezone(&lariv_core::datetime::parse_timezone(timezone))
        .time();
    let window_seconds = i64::from(window_minutes).saturating_mul(60);
    let open = signed_seconds_from(local, work_start).abs() <= window_seconds;
    PunchAvailability {
        punch_in_enabled: open,
        punch_out_enabled: false,
        note: format!(
            "Punch in is available within {} of {}.",
            window_label(window_minutes),
            clock_label(work_start)
        ),
    }
}

pub fn resolve_timezone(subject_tz: &str, viewer_tz: &str) -> String {
    let subject_tz = subject_tz.trim();
    if !subject_tz.is_empty() {
        return subject_tz.to_string();
    }
    let viewer_tz = viewer_tz.trim();
    if !viewer_tz.is_empty() {
        return viewer_tz.to_string();
    }
    lariv_core::datetime::DEFAULT_TIMEZONE.to_string()
}

pub async fn punch_subject(db: &DatabaseConnection, user_id: i64) -> Result<PunchSubject, String> {
    let employee = crate::logic::dashboard::employee_for_user(db, user_id).await;
    let timezone = lariv_core::web::opt_or_log(
        UserEntity::find_by_id(user_id).one(db).await,
        "find user by id",
    )
    .map(|user| user.timezone.as_str().to_string())
    .unwrap_or_default();
    Ok(PunchSubject {
        is_employee: employee.is_some(),
        work_start: employee.and_then(|employee| employee.work_start),
        already_in: open_attendance(db, user_id).await?.is_some(),
        timezone,
    })
}

/// Open an attendance row at the current instant.
///
/// Fails when one is already open, when the user has no work start, or when
/// local time is outside `window_minutes` of that start. `viewer_tz` is used
/// when the employee has no timezone of their own.
pub async fn punch_in(
    db: &DatabaseConnection,
    user_id: i64,
    window_minutes: u32,
    viewer_tz: &str,
) -> Result<attendance::Model, String> {
    let subject = punch_subject(db, user_id).await?;
    if !subject.is_employee {
        return Err("employee is required".to_string());
    }
    let timezone = resolve_timezone(&subject.timezone, viewer_tz);
    let availability = punch_availability(
        Utc::now(),
        &timezone,
        subject.work_start,
        subject.already_in,
        window_minutes,
    );
    if !availability.punch_in_enabled {
        return Err(availability.note);
    }
    let now = Utc::now();
    attendance::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        user_id: Set(user_id),
        started_at: Set(now),
        ended_at: Set(None),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

/// Close the latest open attendance row at the current instant.
pub async fn punch_out(db: &DatabaseConnection, user_id: i64) -> Result<attendance::Model, String> {
    require_employee(db, user_id).await?;
    let existing = open_attendance(db, user_id)
        .await?
        .ok_or_else(|| "no open attendance".to_string())?;
    let now = Utc::now();
    if now <= existing.started_at {
        return Err("end must be after start".to_string());
    }
    let mut am: attendance::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.ended_at = Set(Some(now));
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn delete_attendance(db: &DatabaseConnection, id: i64) -> Result<(), String> {
    AttendanceEntity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveTime, TimeZone, Utc};

    use super::punch_availability;

    fn at(hour: u32, minute: u32, second: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 13, hour, minute, second)
            .unwrap()
    }

    fn start(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
    }

    #[test]
    fn punch_in_is_open_within_fifteen_minutes_of_work_start() {
        let work_start = start(9, 0);
        let open = punch_availability(at(3, 30, 0), "Asia/Kolkata", Some(work_start), false, 15);
        assert!(open.punch_in_enabled);
        assert!(!open.punch_out_enabled);

        let early = punch_availability(at(3, 15, 0), "Asia/Kolkata", Some(work_start), false, 15);
        assert!(early.punch_in_enabled);

        let late = punch_availability(at(3, 45, 0), "Asia/Kolkata", Some(work_start), false, 15);
        assert!(late.punch_in_enabled);
        assert_eq!(
            late.note,
            "Punch in is available within 15 minutes of 09:00."
        );
    }

    #[test]
    fn punch_in_closes_outside_the_window() {
        let work_start = start(9, 0);
        let too_early =
            punch_availability(at(3, 14, 59), "Asia/Kolkata", Some(work_start), false, 15);
        assert!(!too_early.punch_in_enabled);
        let too_late =
            punch_availability(at(3, 45, 1), "Asia/Kolkata", Some(work_start), false, 15);
        assert!(!too_late.punch_in_enabled);
    }

    #[test]
    fn punch_in_window_wraps_midnight() {
        let work_start = start(0, 10);
        let open = punch_availability(at(18, 28, 0), "Asia/Kolkata", Some(work_start), false, 15);
        assert!(open.punch_in_enabled);
        let closed = punch_availability(at(18, 19, 0), "Asia/Kolkata", Some(work_start), false, 15);
        assert!(!closed.punch_in_enabled);
    }

    #[test]
    fn punch_in_stays_closed_without_a_work_start_or_while_already_in() {
        let missing = punch_availability(at(3, 30, 0), "Asia/Kolkata", None, false, 15);
        assert!(!missing.punch_in_enabled);
        assert!(!missing.punch_out_enabled);
        assert_eq!(missing.note, "Set a work start time to punch in.");

        let open_shift =
            punch_availability(at(3, 30, 0), "Asia/Kolkata", Some(start(9, 0)), true, 15);
        assert!(!open_shift.punch_in_enabled);
        assert!(open_shift.punch_out_enabled);
    }

    #[test]
    fn a_wider_window_keeps_punch_in_open() {
        let work_start = start(9, 0);
        let closed = punch_availability(at(3, 0, 0), "Asia/Kolkata", Some(work_start), false, 15);
        assert!(!closed.punch_in_enabled);
        let open = punch_availability(at(3, 0, 0), "Asia/Kolkata", Some(work_start), false, 30);
        assert!(open.punch_in_enabled);
        assert_eq!(
            open.note,
            "Punch in is available within 30 minutes of 09:00."
        );
    }
}
