use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use lariv_core::db::location::{self, GeoPoint, LatLng};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Statement,
};

use crate::entities::attendance::{
    AttendanceColumns, Record,
    postgres::{self as pg},
    sqlite::{self as lite},
};
use crate::logic::preferences::{AttendanceZone, load_attendance_zone};
use lariv_plugin_users::entities::user::{self, Entity as UserEntity};

/// Result of a punch after the employee rules pass.
pub enum PunchOutcome {
    Done(Record),
    MissingLocation,
    Outside {
        point: LatLng,
        zone: AttendanceZone,
        distance_meters: f64,
    },
}

pub struct AttendanceInput {
    pub user_id: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
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
    if let Some(ended_at) = input.ended_at {
        if ended_at <= input.started_at {
            return Err("end must be after start".to_string());
        }
    }
    Ok(())
}

pub async fn create_attendance(
    db: &DatabaseConnection,
    input: AttendanceInput,
) -> Result<Record, String> {
    validate_attendance_input(db, &input).await?;
    let now = Utc::now();
    match db.get_database_backend() {
        DatabaseBackend::Postgres => pg::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            user_id: Set(input.user_id),
            started_at: Set(input.started_at),
            ended_at: Set(input.ended_at),
            started_position: Set(None),
            ended_position: Set(None),
        }
        .insert(db)
        .await
        .map(Into::into)
        .map_err(|e| e.to_string()),
        _ => lite::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            user_id: Set(input.user_id),
            started_at: Set(input.started_at),
            ended_at: Set(input.ended_at),
            started_latitude: Set(None),
            started_longitude: Set(None),
            ended_latitude: Set(None),
            ended_longitude: Set(None),
        }
        .insert(db)
        .await
        .map(Into::into)
        .map_err(|e| e.to_string()),
    }
}

pub async fn update_attendance(
    db: &DatabaseConnection,
    id: i64,
    input: AttendanceInput,
) -> Result<Record, String> {
    validate_attendance_input(db, &input).await?;
    let now = Utc::now();
    match db.get_database_backend() {
        DatabaseBackend::Postgres => {
            let existing = pg::Entity::find_by_id(id)
                .one(db)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "attendance not found".to_string())?;
            let mut am: pg::ActiveModel = existing.into();
            am.updated_at = Set(Some(now));
            am.user_id = Set(input.user_id);
            am.started_at = Set(input.started_at);
            am.ended_at = Set(input.ended_at);
            am.update(db)
                .await
                .map(Into::into)
                .map_err(|e| e.to_string())
        }
        _ => {
            let existing = lite::Entity::find_by_id(id)
                .one(db)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "attendance not found".to_string())?;
            let mut am: lite::ActiveModel = existing.into();
            am.updated_at = Set(Some(now));
            am.user_id = Set(input.user_id);
            am.started_at = Set(input.started_at);
            am.ended_at = Set(input.ended_at);
            am.update(db)
                .await
                .map(Into::into)
                .map_err(|e| e.to_string())
        }
    }
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

async fn open_attendance(db: &DatabaseConnection, user_id: i64) -> Result<Option<Record>, String> {
    match db.get_database_backend() {
        DatabaseBackend::Postgres => open_attendance_on::<pg::Entity>(db, user_id).await,
        _ => open_attendance_on::<lite::Entity>(db, user_id).await,
    }
}

async fn open_attendance_on<E>(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<Option<Record>, String>
where
    E: EntityTrait,
    E::Column: AttendanceColumns,
    E::Model: Into<Record>,
{
    E::find()
        .filter(E::Column::col_user_id().eq(user_id))
        .filter(E::Column::col_ended_at().is_null())
        .order_by_desc(E::Column::col_started_at())
        .order_by_desc(E::Column::col_id())
        .one(db)
        .await
        .map(|row| row.map(Into::into))
        .map_err(|e| e.to_string())
}

/// Closed punches, used by leave calculation. Locations are loaded with the row.
pub async fn closed_attendance_records(db: &DatabaseConnection) -> Result<Vec<Record>, String> {
    match db.get_database_backend() {
        DatabaseBackend::Postgres => closed_on::<pg::Entity>(db).await,
        _ => closed_on::<lite::Entity>(db).await,
    }
}

async fn closed_on<E>(db: &DatabaseConnection) -> Result<Vec<Record>, String>
where
    E: EntityTrait,
    E::Column: AttendanceColumns,
    E::Model: Into<Record>,
{
    E::find()
        .filter(E::Column::col_ended_at().is_not_null())
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(|err| err.to_string())
}

/// Whether the punch buttons can be used for this user right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PunchAvailability {
    pub punch_in_enabled: bool,
    pub punch_out_enabled: bool,
    pub note: String,
}

/// Employee status and open attendance used to decide punch buttons.
pub struct PunchSubject {
    pub is_employee: bool,
    pub already_in: bool,
    /// True when `hr_employees.created_at` is at least 24 hours ago.
    pub employed_long_enough: bool,
}

/// An employee can punch in only after their employee record has existed this long.
pub const MIN_EMPLOYMENT_BEFORE_ATTENDANCE: Duration = Duration::hours(24);

pub fn employment_old_enough(created_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    created_at.is_some_and(|created_at| {
        now.signed_duration_since(created_at) >= MIN_EMPLOYMENT_BEFORE_ATTENDANCE
    })
}

/// Punch in stays closed until the employee record is 24 hours old.
/// An open attendance can still be punched out.
pub fn punch_availability(already_in: bool, employed_long_enough: bool) -> PunchAvailability {
    if already_in {
        PunchAvailability {
            punch_in_enabled: false,
            punch_out_enabled: true,
            note: "Already punched in.".to_string(),
        }
    } else if !employed_long_enough {
        PunchAvailability {
            punch_in_enabled: false,
            punch_out_enabled: false,
            note: "Attendance can be marked 24 hours after the employee record is created."
                .to_string(),
        }
    } else {
        PunchAvailability {
            punch_in_enabled: true,
            punch_out_enabled: false,
            note: String::new(),
        }
    }
}

pub async fn punch_subject(db: &DatabaseConnection, user_id: i64) -> Result<PunchSubject, String> {
    let employee = crate::logic::dashboard::employee_for_user(db, user_id).await;
    let employed_long_enough = employee
        .as_ref()
        .is_some_and(|employee| employment_old_enough(employee.created_at, Utc::now()));
    Ok(PunchSubject {
        is_employee: employee.is_some(),
        already_in: open_attendance(db, user_id).await?.is_some(),
        employed_long_enough,
    })
}

/// Open an attendance row at the current instant.
///
/// Fails when one is already open, when the user is not an employee, or when
/// the employee record is less than 24 hours old. A configured zone also
/// requires `position` to fall inside it.
pub async fn punch_in(
    db: &DatabaseConnection,
    user_id: i64,
    position: Option<LatLng>,
) -> Result<PunchOutcome, String> {
    let subject = punch_subject(db, user_id).await?;
    if !subject.is_employee {
        return Err("employee is required".to_string());
    }
    let availability = punch_availability(subject.already_in, subject.employed_long_enough);
    if !availability.punch_in_enabled {
        return Err(availability.note);
    }
    apply_punch_in(db, user_id, position).await
}

pub(crate) async fn apply_punch_in(
    db: &DatabaseConnection,
    user_id: i64,
    position: Option<LatLng>,
) -> Result<PunchOutcome, String> {
    let position = match evaluate_zone(db, position).await? {
        ZoneDecision::Allow(position) => position,
        ZoneDecision::Missing => return Ok(PunchOutcome::MissingLocation),
        ZoneDecision::Outside {
            point,
            zone,
            distance_meters,
        } => {
            return Ok(PunchOutcome::Outside {
                point,
                zone,
                distance_meters,
            });
        }
    };
    let now = Utc::now();
    let record = match db.get_database_backend() {
        DatabaseBackend::Postgres => pg::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            user_id: Set(user_id),
            started_at: Set(now),
            ended_at: Set(None),
            started_position: Set(position.map(GeoPoint::from)),
            ended_position: Set(None),
        }
        .insert(db)
        .await
        .map(Record::from)
        .map_err(|e| e.to_string())?,
        _ => lite::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            user_id: Set(user_id),
            started_at: Set(now),
            ended_at: Set(None),
            started_latitude: Set(position.map(|point| point.latitude)),
            started_longitude: Set(position.map(|point| point.longitude)),
            ended_latitude: Set(None),
            ended_longitude: Set(None),
        }
        .insert(db)
        .await
        .map(Record::from)
        .map_err(|e| e.to_string())?,
    };
    Ok(PunchOutcome::Done(record))
}

/// Close the latest open attendance row at the current instant.
pub async fn punch_out(
    db: &DatabaseConnection,
    user_id: i64,
    position: Option<LatLng>,
) -> Result<PunchOutcome, String> {
    require_employee(db, user_id).await?;
    let existing = open_attendance(db, user_id)
        .await?
        .ok_or_else(|| "no open attendance".to_string())?;
    let now = Utc::now();
    if now <= existing.started_at {
        return Err("end must be after start".to_string());
    }
    let position = match evaluate_zone(db, position).await? {
        ZoneDecision::Allow(position) => position,
        ZoneDecision::Missing => return Ok(PunchOutcome::MissingLocation),
        ZoneDecision::Outside {
            point,
            zone,
            distance_meters,
        } => {
            return Ok(PunchOutcome::Outside {
                point,
                zone,
                distance_meters,
            });
        }
    };
    let record = match db.get_database_backend() {
        DatabaseBackend::Postgres => {
            let model = pg::Entity::find_by_id(existing.id)
                .one(db)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "attendance not found".to_string())?;
            let mut am: pg::ActiveModel = model.into();
            am.updated_at = Set(Some(now));
            am.ended_at = Set(Some(now));
            am.ended_position = Set(position.map(GeoPoint::from));
            am.update(db)
                .await
                .map(Record::from)
                .map_err(|e| e.to_string())?
        }
        _ => {
            let model = lite::Entity::find_by_id(existing.id)
                .one(db)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "attendance not found".to_string())?;
            let mut am: lite::ActiveModel = model.into();
            am.updated_at = Set(Some(now));
            am.ended_at = Set(Some(now));
            am.ended_latitude = Set(position.map(|point| point.latitude));
            am.ended_longitude = Set(position.map(|point| point.longitude));
            am.update(db)
                .await
                .map(Record::from)
                .map_err(|e| e.to_string())?
        }
    };
    Ok(PunchOutcome::Done(record))
}

pub async fn delete_attendance(db: &DatabaseConnection, id: i64) -> Result<(), String> {
    lite::Entity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

enum ZoneDecision {
    Allow(Option<LatLng>),
    Missing,
    Outside {
        point: LatLng,
        zone: AttendanceZone,
        distance_meters: f64,
    },
}

async fn evaluate_zone(
    db: &DatabaseConnection,
    reported: Option<LatLng>,
) -> Result<ZoneDecision, String> {
    let Some(zone) = load_attendance_zone(db).await? else {
        return Ok(ZoneDecision::Allow(reported));
    };
    let Some(point) = reported else {
        return Ok(ZoneDecision::Missing);
    };
    let (inside, distance_meters) = zone_check(db, zone, point).await?;
    if inside {
        Ok(ZoneDecision::Allow(Some(point)))
    } else {
        Ok(ZoneDecision::Outside {
            point,
            zone,
            distance_meters,
        })
    }
}

async fn zone_check(
    db: &DatabaseConnection,
    zone: AttendanceZone,
    point: LatLng,
) -> Result<(bool, f64), String> {
    if db.get_database_backend() != DatabaseBackend::Postgres {
        let distance_meters = location::distance_meters(point, zone.center);
        return Ok((distance_meters <= zone.range_meters, distance_meters));
    }
    let ewkt = GeoPoint::from(point).ewkt();
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT ST_DWithin(attendance_position, ST_GeogFromText($1), attendance_range_meters) AS inside, ST_Distance(attendance_position, ST_GeogFromText($1)) AS distance_meters FROM hr_preferences WHERE id = 1",
            [ewkt.into()],
        ))
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "attendance location is not configured".to_string())?;
    let inside: Option<bool> = row.try_get("", "inside").map_err(|err| err.to_string())?;
    let distance_meters: Option<f64> = row
        .try_get("", "distance_meters")
        .map_err(|err| err.to_string())?;
    Ok((
        inside.unwrap_or(false),
        distance_meters.unwrap_or_else(|| location::distance_meters(point, zone.center)),
    ))
}

/// Browser coordinates from a punch form. Empty means the browser did not report one.
pub fn parse_reported_position(latitude: &str, longitude: &str) -> Result<Option<LatLng>, String> {
    let latitude = latitude.trim();
    let longitude = longitude.trim();
    if latitude.is_empty() && longitude.is_empty() {
        return Ok(None);
    }
    if latitude.is_empty() || longitude.is_empty() {
        return Err("location is incomplete".to_string());
    }
    let latitude: f64 = latitude
        .parse()
        .map_err(|_| "invalid latitude".to_string())?;
    let longitude: f64 = longitude
        .parse()
        .map_err(|_| "invalid longitude".to_string())?;
    LatLng::new(latitude, longitude).map(Some)
}

pub fn format_position(started: Option<LatLng>, ended: Option<LatLng>) -> String {
    match (started, ended) {
        (None, None) => "—".to_string(),
        (Some(started), None) => format!("In {}", format_latlng(started)),
        (None, Some(ended)) => format!("Out {}", format_latlng(ended)),
        (Some(started), Some(ended)) => {
            format!(
                "In {}\nOut {}",
                format_latlng(started),
                format_latlng(ended)
            )
        }
    }
}

fn format_latlng(point: LatLng) -> String {
    format!("{:.5}, {:.5}", point.latitude, point.longitude)
}

pub fn format_distance(meters: f64) -> String {
    if meters >= 1000.0 {
        format!("{:.1} km", meters / 1000.0)
    } else {
        format!("{:.0} m", meters)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};

    use super::{employment_old_enough, punch_availability};

    #[test]
    fn punch_in_is_open_until_the_employee_punches_in() {
        let open = punch_availability(false, true);
        assert!(open.punch_in_enabled);
        assert!(!open.punch_out_enabled);
        assert!(open.note.is_empty());

        let already_in = punch_availability(true, true);
        assert!(!already_in.punch_in_enabled);
        assert!(already_in.punch_out_enabled);
        assert_eq!(already_in.note, "Already punched in.");
    }

    #[test]
    fn punch_in_waits_until_the_employee_record_is_one_day_old() {
        let waiting = punch_availability(false, false);
        assert!(!waiting.punch_in_enabled);
        assert!(!waiting.punch_out_enabled);
        assert!(waiting.note.contains("24 hours"));

        let already_in = punch_availability(true, false);
        assert!(!already_in.punch_in_enabled);
        assert!(already_in.punch_out_enabled);
    }

    #[test]
    fn employment_is_old_enough_at_exactly_24_hours() {
        let created_at = Utc.with_ymd_and_hms(2026, 10, 19, 12, 0, 0).unwrap();
        let now = created_at + Duration::hours(24);
        assert!(!employment_old_enough(
            Some(created_at),
            now - Duration::seconds(1)
        ));
        assert!(employment_old_enough(Some(created_at), now));
        assert!(!employment_old_enough(None, now));
    }

    #[tokio::test]
    async fn sqlite_punch_stores_coordinates_and_rejects_outside() {
        use sea_orm::{
            ActiveModelTrait, ActiveValue::Set, ConnectionTrait, Database, DatabaseBackend,
            EntityTrait, Schema, Statement,
        };

        use crate::entities::attendance::sqlite as attendance_sqlite;
        use crate::entities::employee;
        use crate::entities::preferences::sqlite as prefs;
        use crate::logic::preferences::save_attendance_zone;
        use lariv_core::db::location::LatLng;

        use super::{PunchOutcome, apply_punch_in, punch_out};

        let db = Database::connect("sqlite::memory:").await.expect("sqlite");
        db.execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "PRAGMA foreign_keys = OFF",
        ))
        .await
        .expect("pragma");
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        for stmt in [
            schema.create_table_from_entity(prefs::Entity),
            schema.create_table_from_entity(attendance_sqlite::Entity),
            schema.create_table_from_entity(employee::Entity),
        ] {
            db.execute_raw(backend.build(&stmt))
                .await
                .expect("create table");
        }
        let created_at = Utc::now() - Duration::hours(48);
        employee::ActiveModel {
            user_id: Set(7),
            employee_code: Set("E00007".to_string()),
            verified: Set(true),
            created_at: Set(Some(created_at)),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("employee");
        let center = LatLng::new(12.9716, 77.5946).unwrap();
        save_attendance_zone(
            &db,
            Some(crate::logic::preferences::AttendanceZone {
                center,
                range_meters: 100.0,
            }),
        )
        .await
        .expect("zone");

        let outside = apply_punch_in(&db, 7, Some(LatLng::new(0.0, 0.0).unwrap()))
            .await
            .expect("outside decision");
        assert!(matches!(outside, PunchOutcome::Outside { .. }));
        assert!(
            attendance_sqlite::Entity::find()
                .all(&db)
                .await
                .unwrap()
                .is_empty(),
            "outside punch must not be stored"
        );

        let inside = apply_punch_in(&db, 7, Some(center))
            .await
            .expect("inside punch");
        let PunchOutcome::Done(row) = inside else {
            panic!("expected a stored punch");
        };
        let started = row.started.expect("started position");
        assert!((started.latitude - center.latitude).abs() < 1e-6);
        assert!((started.longitude - center.longitude).abs() < 1e-6);

        let closed = punch_out(&db, 7, Some(center)).await.expect("punch out");
        let PunchOutcome::Done(closed) = closed else {
            panic!("expected punch out to store");
        };
        assert!(closed.ended.is_some());
    }
}
