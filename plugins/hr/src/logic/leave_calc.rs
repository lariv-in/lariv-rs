//! Earn leave from consecutive perfect attendance.
//!
//! Casual and sick leave are credited on a yearly or monthly schedule. Privilege
//! leave has no schedule: the configured days are added when a streak reaches
//! the consecutive-day count.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc, Weekday};
use chrono_tz::Tz;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    TransactionTrait,
};

use crate::entities::attendance::{self, Entity as AttendanceEntity};
use crate::entities::employee::Entity as EmployeeEntity;
use crate::entities::holiday::Entity as HolidayEntity;
use crate::entities::leaves::leave_attendance_use::{self, Entity as LeaveAttendanceUseEntity};
use crate::entities::leaves::leave_calc_preference::{self, Entity as LeaveCalcPreferenceEntity};
use crate::entities::leaves::leave_evaluation_run::{self, Entity as LeaveEvaluationRunEntity};
use crate::entities::leaves::leave_type::LeaveType;
use crate::entities::leaves::{ApprovedLeaveEntity, LeaveApplicationEntity};
use crate::forms::LeaveCalcPreferencesForm;
use crate::logic::leave::append_leave_journal;

use lariv_core::datetime::DEFAULT_TIMEZONE;

pub const SCHEDULE_MONTHLY: &str = "monthly";
pub const SCHEDULE_YEARLY: &str = "yearly";
pub const DAY_LAST: &str = "last";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScheduleKind {
    Monthly,
    Yearly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaySpec {
    Day(u8),
    Last,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeaveSchedule {
    pub kind: ScheduleKind,
    pub month: u8,
    pub day: DaySpec,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeaveTypeCalc {
    pub leave_type: LeaveType,
    pub consecutive_required: i64,
    pub leave_allocated: i64,
    /// Absent for privilege leave, which is credited as soon as the streak is long enough.
    pub schedule: Option<LeaveSchedule>,
    pub timezone: String,
}

impl LeaveTypeCalc {
    pub fn is_active(&self) -> bool {
        self.consecutive_required > 0 && self.leave_allocated > 0
    }
}

fn parse_leave_allocated(raw: &str, label: &str) -> Result<i64, String> {
    let leave_allocated = raw
        .trim()
        .parse::<i64>()
        .map_err(|_err| format!("{label}: leave allocated must be a whole number"))?;
    if leave_allocated < 0 {
        return Err(format!("{label}: leave allocated must be at least 0"));
    }
    Ok(leave_allocated)
}

pub fn timezone_is_valid(tz: &str) -> bool {
    tz.trim().parse::<Tz>().is_ok()
}

pub fn parse_schedule_kind(raw: &str) -> Result<ScheduleKind, String> {
    match raw.trim() {
        SCHEDULE_MONTHLY => Ok(ScheduleKind::Monthly),
        SCHEDULE_YEARLY => Ok(ScheduleKind::Yearly),
        other => Err(format!("unknown schedule {other:?}")),
    }
}

pub fn parse_day_spec(raw: &str) -> Result<DaySpec, String> {
    let raw = raw.trim();
    if raw.eq_ignore_ascii_case(DAY_LAST) {
        return Ok(DaySpec::Last);
    }
    let day = raw
        .parse::<u8>()
        .map_err(|_err| format!("day must be 1–31 or {DAY_LAST}"))?;
    if !(1..=31).contains(&day) {
        return Err(format!("day must be 1–31 or {DAY_LAST}"));
    }
    Ok(DaySpec::Day(day))
}

pub fn parse_month(raw: &str) -> Result<u8, String> {
    let month = raw
        .trim()
        .parse::<u8>()
        .map_err(|_err| "month must be 1–12".to_string())?;
    if !(1..=12).contains(&month) {
        return Err("month must be 1–12".to_string());
    }
    Ok(month)
}

/// Reject dates that never occur, such as 31 February. 29 February is allowed.
pub fn validate_schedule(schedule: LeaveSchedule) -> Result<(), String> {
    match (schedule.kind, schedule.day) {
        (ScheduleKind::Monthly, DaySpec::Last) | (ScheduleKind::Yearly, DaySpec::Last) => Ok(()),
        (ScheduleKind::Monthly, DaySpec::Day(day)) => {
            if (1..=31).contains(&day) {
                Ok(())
            } else {
                Err("day must be 1–31 or last".to_string())
            }
        }
        (ScheduleKind::Yearly, DaySpec::Day(day)) => {
            if !(1..=12).contains(&schedule.month) {
                return Err("month must be 1–12".to_string());
            }
            if NaiveDate::from_ymd_opt(2024, schedule.month as u32, day as u32).is_none() {
                return Err(format!(
                    "day {day} is not a day of month {}",
                    schedule.month
                ));
            }
            Ok(())
        }
    }
}

/// Period key when today's local date is on or after this period's scheduled day.
///
/// A monthly day of 31 is not due in shorter months. Yearly Last is 31 December.
/// An earlier period that was never run is not backfilled; its unused attendance
/// stays available for the next run.
pub fn due_period(schedule: LeaveSchedule, today: NaiveDate) -> Option<String> {
    let year = today.year();
    let month = today.month();
    match schedule.kind {
        ScheduleKind::Monthly => {
            let scheduled = scheduled_day(year, month, schedule.day)?;
            if today >= scheduled {
                Some(format!("monthly:{year}-{month:02}"))
            } else {
                None
            }
        }
        ScheduleKind::Yearly => {
            let scheduled = match schedule.day {
                DaySpec::Last => NaiveDate::from_ymd_opt(year, 12, 31)?,
                DaySpec::Day(day) => {
                    NaiveDate::from_ymd_opt(year, schedule.month as u32, day as u32)?
                }
            };
            if today >= scheduled {
                Some(format!("yearly:{year}"))
            } else {
                None
            }
        }
    }
}

fn scheduled_day(year: i32, month: u32, day: DaySpec) -> Option<NaiveDate> {
    match day {
        DaySpec::Last => last_day_of_month(year, month),
        DaySpec::Day(day) => NaiveDate::from_ymd_opt(year, month, day as u32),
    }
}

fn last_day_of_month(year: i32, month: u32) -> Option<NaiveDate> {
    let first_next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)?
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)?
    };
    Some(first_next - chrono::Duration::days(1))
}

/// First local date to walk. Days on or before the latest used date are excluded.
pub fn range_start(
    used: &HashSet<NaiveDate>,
    closed: &HashSet<NaiveDate>,
    today: NaiveDate,
) -> Option<NaiveDate> {
    if let Some(last_used) = used.iter().copied().max() {
        let start = last_used + chrono::Duration::days(1);
        (start < today).then_some(start)
    } else {
        closed.iter().copied().filter(|date| *date < today).min()
    }
}

/// Qualifying streaks of perfect weekdays in `[start, today)`.
///
/// Saturday, Sunday, and holidays are skipped. A weekday without a closed punch,
/// or with an approved leave, breaks the streak. Streaks shorter than `threshold`
/// are left unused.
pub fn collect_streaks(
    start: NaiveDate,
    today: NaiveDate,
    closed: &HashSet<NaiveDate>,
    holidays: &HashSet<NaiveDate>,
    approved: &HashSet<NaiveDate>,
    threshold: i64,
) -> Vec<Vec<NaiveDate>> {
    if threshold <= 0 || start >= today {
        return Vec::new();
    }
    let mut streaks = Vec::new();
    let mut current = Vec::new();
    let mut date = start;
    while date < today {
        let skipped = date.weekday() == Weekday::Sat
            || date.weekday() == Weekday::Sun
            || holidays.contains(&date);
        if skipped {
            date += chrono::Duration::days(1);
            continue;
        }
        let perfect = closed.contains(&date) && !approved.contains(&date);
        if perfect {
            current.push(date);
        } else if !current.is_empty() {
            push_if_ready(&mut streaks, &mut current, threshold);
        }
        date += chrono::Duration::days(1);
    }
    if !current.is_empty() {
        push_if_ready(&mut streaks, &mut current, threshold);
    }
    streaks
}

fn push_if_ready(streaks: &mut Vec<Vec<NaiveDate>>, current: &mut Vec<NaiveDate>, threshold: i64) {
    if current.len() as i64 >= threshold {
        streaks.push(std::mem::take(current));
    } else {
        current.clear();
    }
}

/// Complete privilege-leave grants inside one streak.
///
/// Each chunk is exactly `consecutive_required` perfect days and earns one
/// allocation. Days after the last complete chunk stay unused.
pub fn privilege_grant_chunks(
    streak: &[NaiveDate],
    consecutive_required: i64,
) -> Vec<Vec<NaiveDate>> {
    if consecutive_required <= 0 {
        return Vec::new();
    }
    let width = consecutive_required as usize;
    let complete = streak.len() / width * width;
    streak[..complete]
        .chunks(width)
        .map(|chunk| chunk.to_vec())
        .collect()
}

pub fn duration_until_next_hour(now: DateTime<Utc>) -> Duration {
    let next = (now + chrono::Duration::hours(1))
        .with_minute(0)
        .and_then(|dt| dt.with_second(0))
        .and_then(|dt| dt.with_nanosecond(0))
        .unwrap_or(now + chrono::Duration::hours(1));
    (next - now).to_std().unwrap_or(Duration::from_secs(3600))
}

pub fn schedule_from_row(
    kind: &str,
    month: Option<i32>,
    day_spec: &str,
) -> Result<LeaveSchedule, String> {
    let kind = parse_schedule_kind(kind)?;
    let day = parse_day_spec(day_spec)?;
    let month = match month {
        Some(value) => {
            let month = u8::try_from(value).map_err(|_err| "month must be 1–12".to_string())?;
            if !(1..=12).contains(&month) {
                return Err("month must be 1–12".to_string());
            }
            month
        }
        None => 1,
    };
    let schedule = LeaveSchedule { kind, month, day };
    validate_schedule(schedule)?;
    Ok(schedule)
}

pub fn parse_preferences_form(
    form: &LeaveCalcPreferencesForm,
) -> Result<[LeaveTypeCalc; 3], String> {
    let timezone = form.timezone.trim();
    if !timezone_is_valid(timezone) {
        return Err("timezone must be an IANA name, such as Asia/Kolkata".to_string());
    }
    let timezone = timezone.to_string();
    Ok([
        parse_type_fields(
            LeaveType::Casual,
            None,
            &form.casual_leave_allocated,
            &form.casual_schedule_kind,
            &form.casual_month,
            &form.casual_day,
            &timezone,
        )?,
        parse_type_fields(
            LeaveType::Sick,
            None,
            &form.sick_leave_allocated,
            &form.sick_schedule_kind,
            &form.sick_month,
            &form.sick_day,
            &timezone,
        )?,
        parse_privilege_fields(
            &form.privilege_consecutive_required,
            &form.privilege_leave_allocated,
            &timezone,
        )?,
    ])
}

fn parse_privilege_fields(
    consecutive_required: &str,
    leave_allocated: &str,
    timezone: &str,
) -> Result<LeaveTypeCalc, String> {
    let label = LeaveType::Privilege.label();
    let leave_allocated = parse_leave_allocated(leave_allocated, label)?;
    let consecutive_required = consecutive_required
        .trim()
        .parse::<i64>()
        .map_err(|_err| format!("{label}: consecutive days must be a whole number"))?;
    if consecutive_required < 0 {
        return Err(format!("{label}: consecutive days must be at least 0"));
    }
    Ok(LeaveTypeCalc {
        leave_type: LeaveType::Privilege,
        consecutive_required,
        leave_allocated,
        schedule: None,
        timezone: timezone.to_string(),
    })
}

fn parse_type_fields(
    leave_type: LeaveType,
    consecutive_required: Option<&str>,
    leave_allocated: &str,
    schedule_kind: &str,
    month: &str,
    day: &str,
    timezone: &str,
) -> Result<LeaveTypeCalc, String> {
    let label = leave_type.label();
    let leave_allocated = parse_leave_allocated(leave_allocated, label)?;
    // Casual and sick have no threshold: any streak of perfect days is eligible.
    let consecutive_required = match consecutive_required {
        None => i64::from(leave_allocated > 0),
        Some(raw) => {
            let consecutive_required = raw
                .trim()
                .parse::<i64>()
                .map_err(|_err| format!("{label}: consecutive days must be a whole number"))?;
            if consecutive_required < 0 {
                return Err(format!("{label}: consecutive days must be at least 0"));
            }
            consecutive_required
        }
    };
    let kind = parse_schedule_kind(schedule_kind).map_err(|err| format!("{label}: {err}"))?;
    let month = if kind == ScheduleKind::Yearly {
        Some(i32::from(
            parse_month(month).map_err(|err| format!("{label}: {err}"))?,
        ))
    } else {
        None
    };
    let schedule =
        schedule_from_row(schedule_kind, month, day).map_err(|err| format!("{label}: {err}"))?;
    Ok(LeaveTypeCalc {
        leave_type,
        consecutive_required,
        leave_allocated,
        schedule: Some(schedule),
        timezone: timezone.to_string(),
    })
}

fn default_preference(leave_type: LeaveType) -> leave_calc_preference::ActiveModel {
    let now = Utc::now();
    leave_calc_preference::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        leave_type: Set(leave_type),
        consecutive_required: Set(0),
        leave_allocated: Set(0),
        schedule_kind: Set(SCHEDULE_MONTHLY.to_string()),
        month: Set(Some(1)),
        day_spec: Set("1".to_string()),
        timezone: Set(DEFAULT_TIMEZONE.to_string()),
    }
}

pub async fn load_preferences(
    db: &DatabaseConnection,
) -> Result<[leave_calc_preference::Model; 3], String> {
    let rows = LeaveCalcPreferenceEntity::find()
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut loaded = Vec::with_capacity(3);
    for leave_type in [LeaveType::Casual, LeaveType::Sick, LeaveType::Privilege] {
        if let Some(row) = rows.iter().find(|row| row.leave_type == leave_type) {
            loaded.push(row.clone());
            continue;
        }
        let inserted = default_preference(leave_type)
            .insert(db)
            .await
            .map_err(|err| err.to_string())?;
        loaded.push(inserted);
    }
    let [casual, sick, privilege] = loaded
        .try_into()
        .map_err(|_rows| "leave preferences were not loaded".to_string())?;
    Ok([casual, sick, privilege])
}

pub async fn save_preferences(
    db: &DatabaseConnection,
    form: &LeaveCalcPreferencesForm,
) -> Result<(), String> {
    let parsed = parse_preferences_form(form)?;
    let existing = load_preferences(db).await?;
    let now = Utc::now();
    for input in parsed {
        let Some(row) = existing
            .iter()
            .find(|row| row.leave_type == input.leave_type)
        else {
            continue;
        };
        let mut active: leave_calc_preference::ActiveModel = row.clone().into();
        active.consecutive_required = Set(input.consecutive_required);
        active.leave_allocated = Set(input.leave_allocated);
        if let Some(schedule) = input.schedule {
            active.schedule_kind = Set(match schedule.kind {
                ScheduleKind::Monthly => SCHEDULE_MONTHLY,
                ScheduleKind::Yearly => SCHEDULE_YEARLY,
            }
            .to_string());
            active.month = Set(match schedule.kind {
                ScheduleKind::Yearly => Some(i32::from(schedule.month)),
                ScheduleKind::Monthly => None,
            });
            active.day_spec = Set(match schedule.day {
                DaySpec::Last => DAY_LAST.to_string(),
                DaySpec::Day(day) => day.to_string(),
            });
        }
        active.timezone = Set(input.timezone);
        active.updated_at = Set(Some(now));
        active.update(db).await.map_err(|err| err.to_string())?;
    }
    Ok(())
}

/// Privilege leave uses the stored threshold. Casual and sick count any streak.
fn threshold_for(row: &leave_calc_preference::Model) -> i64 {
    match row.leave_type {
        LeaveType::Privilege => row.consecutive_required,
        LeaveType::Casual | LeaveType::Sick => i64::from(row.leave_allocated > 0),
    }
}

pub async fn evaluate_due_leaves(db: &DatabaseConnection) -> Result<(), String> {
    evaluate_due_leaves_at(db, Utc::now()).await
}

pub async fn evaluate_due_leaves_at(
    db: &DatabaseConnection,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let prefs = LeaveCalcPreferenceEntity::find()
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    for row in prefs {
        let scheduled = row.leave_type != LeaveType::Privilege;
        let schedule = if scheduled {
            match schedule_from_row(&row.schedule_kind, row.month, &row.day_spec) {
                Ok(schedule) => Some(schedule),
                Err(err) => {
                    tracing::error!(
                        target: "hr_leave_calc",
                        leave_type = row.leave_type.as_str(),
                        "skipping leave calculation: {err}"
                    );
                    continue;
                }
            }
        } else {
            None
        };
        let calc = LeaveTypeCalc {
            leave_type: row.leave_type,
            consecutive_required: threshold_for(&row),
            leave_allocated: row.leave_allocated,
            schedule,
            timezone: row.timezone,
        };
        if !calc.is_active() {
            continue;
        }
        let Ok(tz) = calc.timezone.trim().parse::<Tz>() else {
            tracing::error!(
                target: "hr_leave_calc",
                leave_type = calc.leave_type.as_str(),
                timezone = calc.timezone,
                "skipping leave calculation: unknown timezone"
            );
            continue;
        };
        let today = now.with_timezone(&tz).date_naive();
        if let Some(schedule) = calc.schedule {
            let Some(period) = due_period(schedule, today) else {
                continue;
            };
            if period_already_run(db, calc.leave_type, &period).await? {
                continue;
            }
            evaluate_leave_type(db, &calc, today, tz, now).await?;
            record_period(db, calc.leave_type, &period).await?;
        } else {
            evaluate_leave_type(db, &calc, today, tz, now).await?;
        }
    }
    Ok(())
}

async fn period_already_run(
    db: &DatabaseConnection,
    leave_type: LeaveType,
    period: &str,
) -> Result<bool, String> {
    let existing = LeaveEvaluationRunEntity::find()
        .filter(leave_evaluation_run::Column::LeaveType.eq(leave_type))
        .filter(leave_evaluation_run::Column::PeriodKey.eq(period))
        .one(db)
        .await
        .map_err(|err| err.to_string())?;
    Ok(existing.is_some())
}

async fn record_period(
    db: &DatabaseConnection,
    leave_type: LeaveType,
    period: &str,
) -> Result<(), String> {
    let now = Utc::now();
    leave_evaluation_run::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        leave_type: Set(leave_type),
        period_key: Set(period.to_string()),
    }
    .insert(db)
    .await
    .map(|_| ())
    .or_else(|err| {
        let message = err.to_string().to_ascii_lowercase();
        if message.contains("unique") || message.contains("duplicate") {
            Ok(())
        } else {
            Err(err.to_string())
        }
    })
}

async fn evaluate_leave_type(
    db: &DatabaseConnection,
    calc: &LeaveTypeCalc,
    today: NaiveDate,
    tz: Tz,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let employees = EmployeeEntity::find()
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let holidays = holiday_dates(db).await?;
    let closed = closed_attendance_dates(db, tz).await?;
    let approved = approved_leave_dates(db).await?;
    let used = used_dates(db, calc.leave_type).await?;

    for employee in employees {
        let user_id = employee.user_id;
        let empty = HashSet::new();
        let closed_days = closed.get(&user_id).unwrap_or(&empty);
        let approved_days = approved.get(&user_id).unwrap_or(&empty);
        let used_days = used.get(&user_id).unwrap_or(&empty);
        let Some(start) = range_start(used_days, closed_days, today) else {
            continue;
        };
        let streaks = collect_streaks(
            start,
            today,
            closed_days,
            &holidays,
            approved_days,
            calc.consecutive_required,
        );
        for streak in streaks {
            if calc.leave_allocated <= 0 {
                continue;
            }
            let grants = if calc.leave_type == LeaveType::Privilege {
                privilege_grant_chunks(&streak, calc.consecutive_required)
            } else {
                vec![streak]
            };
            for grant in grants {
                credit_streak(
                    db,
                    user_id,
                    calc.leave_type,
                    calc.leave_allocated,
                    &grant,
                    now,
                )
                .await?;
            }
        }
    }
    Ok(())
}

async fn holiday_dates(db: &DatabaseConnection) -> Result<HashSet<NaiveDate>, String> {
    let rows = HolidayEntity::find()
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    Ok(rows.into_iter().map(|row| row.date).collect())
}

async fn closed_attendance_dates(
    db: &DatabaseConnection,
    tz: Tz,
) -> Result<HashMap<i64, HashSet<NaiveDate>>, String> {
    let rows = AttendanceEntity::find()
        .filter(attendance::Column::EndedAt.is_not_null())
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut by_user: HashMap<i64, HashSet<NaiveDate>> = HashMap::new();
    for row in rows {
        let date = row.started_at.with_timezone(&tz).date_naive();
        by_user.entry(row.user_id).or_default().insert(date);
    }
    Ok(by_user)
}

async fn approved_leave_dates(
    db: &DatabaseConnection,
) -> Result<HashMap<i64, HashSet<NaiveDate>>, String> {
    let rows = LeaveApplicationEntity::find()
        .find_with_related(ApprovedLeaveEntity)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut by_user: HashMap<i64, HashSet<NaiveDate>> = HashMap::new();
    for (application, approvals) in rows {
        if approvals.is_empty() {
            continue;
        }
        by_user
            .entry(application.applied_by_id)
            .or_default()
            .insert(application.date);
    }
    Ok(by_user)
}

async fn used_dates(
    db: &DatabaseConnection,
    leave_type: LeaveType,
) -> Result<HashMap<i64, HashSet<NaiveDate>>, String> {
    let rows = LeaveAttendanceUseEntity::find()
        .filter(leave_attendance_use::Column::LeaveType.eq(leave_type))
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut by_user: HashMap<i64, HashSet<NaiveDate>> = HashMap::new();
    for row in rows {
        by_user
            .entry(row.user_id)
            .or_default()
            .insert(row.attendance_date);
    }
    Ok(by_user)
}

async fn credit_streak(
    db: &DatabaseConnection,
    user_id: i64,
    leave_type: LeaveType,
    days: i64,
    streak: &[NaiveDate],
    at: DateTime<Utc>,
) -> Result<(), String> {
    let txn = db.begin().await.map_err(|err| err.to_string())?;
    let now = Utc::now();
    for date in streak {
        leave_attendance_use::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            user_id: Set(user_id),
            leave_type: Set(leave_type),
            attendance_date: Set(*date),
        }
        .insert(&txn)
        .await
        .map_err(|err| err.to_string())?;
    }
    append_leave_journal(&txn, user_id, leave_type, days, at).await?;
    txn.commit().await.map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn set(dates: &[NaiveDate]) -> HashSet<NaiveDate> {
        dates.iter().copied().collect()
    }

    #[test]
    fn monthly_last_day_is_due_on_that_day() {
        let schedule = LeaveSchedule {
            kind: ScheduleKind::Monthly,
            month: 1,
            day: DaySpec::Last,
        };
        assert_eq!(due_period(schedule, date(2026, 2, 27)), None);
        assert_eq!(
            due_period(schedule, date(2026, 2, 28)),
            Some("monthly:2026-02".to_string())
        );
        assert_eq!(
            due_period(schedule, date(2024, 2, 29)),
            Some("monthly:2024-02".to_string())
        );
        assert_eq!(due_period(schedule, date(2026, 10, 30)), None);
        assert_eq!(
            due_period(schedule, date(2026, 10, 31)),
            Some("monthly:2026-10".to_string())
        );
    }

    #[test]
    fn monthly_day_31_skips_shorter_months() {
        let schedule = LeaveSchedule {
            kind: ScheduleKind::Monthly,
            month: 1,
            day: DaySpec::Day(31),
        };
        assert_eq!(due_period(schedule, date(2026, 4, 30)), None);
        assert_eq!(
            due_period(schedule, date(2026, 3, 31)),
            Some("monthly:2026-03".to_string())
        );
    }

    #[test]
    fn yearly_last_is_31_december() {
        let schedule = LeaveSchedule {
            kind: ScheduleKind::Yearly,
            month: 6,
            day: DaySpec::Last,
        };
        assert_eq!(due_period(schedule, date(2026, 12, 30)), None);
        assert_eq!(
            due_period(schedule, date(2026, 12, 31)),
            Some("yearly:2026".to_string())
        );
    }

    #[test]
    fn yearly_month_and_day() {
        let schedule = LeaveSchedule {
            kind: ScheduleKind::Yearly,
            month: 3,
            day: DaySpec::Day(15),
        };
        assert_eq!(due_period(schedule, date(2026, 3, 14)), None);
        assert_eq!(
            due_period(schedule, date(2026, 3, 15)),
            Some("yearly:2026".to_string())
        );
    }

    #[test]
    fn yearly_rejects_impossible_dates() {
        assert!(
            validate_schedule(LeaveSchedule {
                kind: ScheduleKind::Yearly,
                month: 2,
                day: DaySpec::Day(31),
            })
            .is_err()
        );
        assert!(
            validate_schedule(LeaveSchedule {
                kind: ScheduleKind::Yearly,
                month: 2,
                day: DaySpec::Day(29),
            })
            .is_ok()
        );
        assert!(
            validate_schedule(LeaveSchedule {
                kind: ScheduleKind::Monthly,
                month: 1,
                day: DaySpec::Day(31),
            })
            .is_ok()
        );
    }

    #[test]
    fn weekends_and_holidays_do_not_break_or_count() {
        let monday = date(2026, 3, 2);
        assert_eq!(monday.weekday(), Weekday::Mon);
        let days = [
            date(2026, 3, 2),
            date(2026, 3, 3),
            date(2026, 3, 4),
            date(2026, 3, 5),
            date(2026, 3, 6),
            date(2026, 3, 9),
        ];
        let closed = set(&days);
        let holidays = set(&[date(2026, 3, 4)]);
        let streaks = collect_streaks(
            monday,
            date(2026, 3, 10),
            &closed,
            &holidays,
            &HashSet::new(),
            5,
        );
        assert_eq!(
            streaks,
            vec![vec![
                date(2026, 3, 2),
                date(2026, 3, 3),
                date(2026, 3, 5),
                date(2026, 3, 6),
                date(2026, 3, 9),
            ]]
        );
    }

    #[test]
    fn missing_punch_and_approved_leave_break_the_streak() {
        let start = date(2026, 3, 2);
        let closed = set(&[
            date(2026, 3, 2),
            date(2026, 3, 3),
            date(2026, 3, 5),
            date(2026, 3, 6),
        ]);
        let broken = collect_streaks(
            start,
            date(2026, 3, 10),
            &closed,
            &HashSet::new(),
            &HashSet::new(),
            2,
        );
        assert_eq!(
            broken,
            vec![
                vec![date(2026, 3, 2), date(2026, 3, 3)],
                vec![date(2026, 3, 5), date(2026, 3, 6)],
            ]
        );

        let approved = set(&[date(2026, 3, 4)]);
        let with_leave = collect_streaks(
            start,
            date(2026, 3, 10),
            &set(&[
                date(2026, 3, 2),
                date(2026, 3, 3),
                date(2026, 3, 4),
                date(2026, 3, 5),
                date(2026, 3, 6),
            ]),
            &HashSet::new(),
            &approved,
            2,
        );
        assert_eq!(
            with_leave,
            vec![
                vec![date(2026, 3, 2), date(2026, 3, 3)],
                vec![date(2026, 3, 5), date(2026, 3, 6)],
            ]
        );
    }

    #[test]
    fn used_dates_are_excluded_from_the_next_streak() {
        let used = set(&[date(2026, 3, 6)]);
        let closed = set(&[
            date(2026, 3, 2),
            date(2026, 3, 3),
            date(2026, 3, 4),
            date(2026, 3, 5),
            date(2026, 3, 6),
            date(2026, 3, 9),
            date(2026, 3, 10),
        ]);
        let today = date(2026, 3, 11);
        let start = range_start(&used, &closed, today).unwrap();
        assert_eq!(start, date(2026, 3, 7));
        let streaks = collect_streaks(start, today, &closed, &HashSet::new(), &HashSet::new(), 2);
        assert_eq!(streaks, vec![vec![date(2026, 3, 9), date(2026, 3, 10)]]);
    }

    #[test]
    fn privilege_grants_each_complete_streak_and_leaves_the_rest() {
        let days = [
            date(2026, 3, 2),
            date(2026, 3, 3),
            date(2026, 3, 4),
            date(2026, 3, 5),
            date(2026, 3, 6),
        ];
        let chunks = privilege_grant_chunks(&days, 2);
        assert_eq!(
            chunks,
            vec![
                vec![date(2026, 3, 2), date(2026, 3, 3)],
                vec![date(2026, 3, 4), date(2026, 3, 5)],
            ]
        );
        assert!(privilege_grant_chunks(&days, 0).is_empty());
    }

    #[test]
    fn leave_allocated_rejects_non_integers() {
        assert!(parse_leave_allocated("2", "Casual").is_ok());
        assert!(parse_leave_allocated("0", "Casual").is_ok());
        assert!(parse_leave_allocated("-1", "Casual").is_err());
        assert!(parse_leave_allocated("1.5", "Casual").is_err());
    }

    #[test]
    fn next_hour_wait_lands_on_the_hour() {
        let now = DateTime::parse_from_rfc3339("2026-10-09T10:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let wait = duration_until_next_hour(now);
        assert_eq!(wait, Duration::from_secs(30 * 60));
    }
}
