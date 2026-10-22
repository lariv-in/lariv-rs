//! Singleton HR preferences (`id = 1`).

use chrono::Utc;
use lariv_core::db::location::LatLng;
use p256::ecdsa::SigningKey;
use p256::elliptic_curve::Generate;
use p256::pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseConnection, EntityTrait, ExprTrait, QueryFilter, sea_query::Expr,
};

use crate::entities::preferences::{
    HrPreferences,
    postgres::{self as pg},
    sqlite::{self as lite},
};

/// A blank value is allowed. Anything else must be a PKCS#8 PEM P-256 private key.
pub fn validate_id_card_signing_key(pem: &str) -> Result<(), String> {
    let pem = pem.trim();
    if pem.is_empty() {
        return Ok(());
    }
    SigningKey::from_pkcs8_pem(pem)
        .map(|_| ())
        .map_err(|err| format!("ID card signing key must be a PKCS#8 PEM P-256 private key: {err}"))
}

/// Load the singleton preferences row, creating it when missing.
pub async fn load_hr_preferences<C: ConnectionTrait>(db: &C) -> Result<HrPreferences, String> {
    match db.get_database_backend() {
        DatabaseBackend::Postgres => {
            if let Some(row) = pg::Entity::find_by_id(1)
                .one(db)
                .await
                .map_err(|err| err.to_string())?
            {
                return Ok(row.into());
            }
            let now = Utc::now();
            pg::ActiveModel {
                id: Set(1),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                employee_id_format: Set(None),
                id_card_format: Set(None),
                id_card_signing_key: Set(None),
                attendance_range_meters: Set(None),
                attendance_position: Set(None),
            }
            .insert(db)
            .await
            .map(Into::into)
            .map_err(|err| err.to_string())
        }
        _ => {
            if let Some(row) = lite::Entity::find_by_id(1)
                .one(db)
                .await
                .map_err(|err| err.to_string())?
            {
                return Ok(row.into());
            }
            let now = Utc::now();
            lite::ActiveModel {
                id: Set(1),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                employee_id_format: Set(None),
                id_card_format: Set(None),
                id_card_signing_key: Set(None),
                attendance_range_meters: Set(None),
                attendance_latitude: Set(None),
                attendance_longitude: Set(None),
            }
            .insert(db)
            .await
            .map(Into::into)
            .map_err(|err| err.to_string())
        }
    }
}

/// Saved employee ID format. Empty means the default joining-year code.
pub async fn load_employee_id_format<C: ConnectionTrait>(db: &C) -> Result<String, String> {
    Ok(load_hr_preferences(db)
        .await?
        .employee_id_format
        .unwrap_or_default())
}

/// Persist the employee ID format. A blank value restores the default.
pub async fn save_employee_id_format(db: &DatabaseConnection, format: &str) -> Result<(), String> {
    crate::logic::employee_code::validate_employee_id_format(format)?;
    let format = format.trim();
    let value = if format.is_empty() {
        None
    } else {
        Some(format.to_string())
    };
    save_text_field(db, TextPref::EmployeeId(value)).await
}

/// Saved ID card template. Empty means the shipped example.
pub async fn load_id_card_format(db: &DatabaseConnection) -> Result<String, String> {
    Ok(load_hr_preferences(db)
        .await?
        .id_card_format
        .unwrap_or_default())
}

/// Persist the ID card template. A blank value restores the shipped example.
pub async fn save_id_card_format(db: &DatabaseConnection, format: &str) -> Result<(), String> {
    let format = format.trim();
    let value = if format.is_empty() {
        None
    } else {
        Some(format.to_string())
    };
    save_text_field(db, TextPref::IdCard(value)).await
}

/// Saved ID card signing key. Empty means none has been stored yet.
pub async fn load_id_card_signing_key(db: &DatabaseConnection) -> Result<String, String> {
    Ok(load_hr_preferences(db)
        .await?
        .id_card_signing_key
        .unwrap_or_default())
}

/// PKCS#8 PEM for a new P-256 signing key.
pub fn generate_id_card_signing_key() -> Result<String, String> {
    let key =
        SigningKey::try_generate().map_err(|err| format!("generate ID card signing key: {err}"))?;
    key.to_pkcs8_pem(LineEnding::LF)
        .map(|pem| pem.to_string())
        .map_err(|err| format!("encode ID card signing key: {err}"))
}

/// Return the stored signing key, generating and saving one when it is empty.
pub async fn ensure_id_card_signing_key(db: &DatabaseConnection) -> Result<String, String> {
    let prefs = load_hr_preferences(db).await?;
    if let Some(pem) = prefs
        .id_card_signing_key
        .as_deref()
        .map(str::trim)
        .filter(|pem| !pem.is_empty())
    {
        validate_id_card_signing_key(pem)?;
        return Ok(pem.to_string());
    }
    let pem = generate_id_card_signing_key()?;
    let updated = lite::Entity::update_many()
        .col_expr(lite::Column::IdCardSigningKey, Expr::value(pem.clone()))
        .col_expr(lite::Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(lite::Column::Id.eq(1))
        .filter(
            lite::Column::IdCardSigningKey
                .is_null()
                .or(lite::Column::IdCardSigningKey.eq("")),
        )
        .exec(db)
        .await
        .map_err(|err| err.to_string())?;
    if updated.rows_affected == 0 {
        let stored = load_id_card_signing_key(db).await?;
        let stored = stored.trim();
        if stored.is_empty() {
            return Err("ID card signing key was not saved".to_string());
        }
        validate_id_card_signing_key(stored)?;
        return Ok(stored.to_string());
    }
    Ok(pem)
}

/// Persist the ID card signing key. A blank value stores a newly generated key.
pub async fn save_id_card_signing_key(db: &DatabaseConnection, pem: &str) -> Result<(), String> {
    let pem = if pem.trim().is_empty() {
        generate_id_card_signing_key()?
    } else {
        validate_id_card_signing_key(pem)?;
        pem.trim().to_string()
    };
    save_text_field(db, TextPref::SigningKey(Some(pem))).await
}

/// Office location and radius. `None` means punches are not geofenced.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttendanceZone {
    pub center: LatLng,
    pub range_meters: f64,
}

const MIN_RANGE_METERS: f64 = 1.0;
const MAX_RANGE_METERS: f64 = 100_000.0;

/// Empty latitude, longitude, and range clear the zone. A partial value is rejected.
pub fn parse_attendance_zone(
    latitude: &str,
    longitude: &str,
    range_meters: &str,
) -> Result<Option<AttendanceZone>, String> {
    let latitude = latitude.trim();
    let longitude = longitude.trim();
    let range_meters = range_meters.trim();
    if latitude.is_empty() && longitude.is_empty() && range_meters.is_empty() {
        return Ok(None);
    }
    if latitude.is_empty() || longitude.is_empty() || range_meters.is_empty() {
        return Err(
            "Set the attendance location and range together, or leave all three empty.".to_string(),
        );
    }
    let latitude: f64 = latitude
        .parse()
        .map_err(|_| "attendance latitude is invalid".to_string())?;
    let longitude: f64 = longitude
        .parse()
        .map_err(|_| "attendance longitude is invalid".to_string())?;
    let range: f64 = range_meters
        .parse()
        .map_err(|_| "attendance range is invalid".to_string())?;
    let center = LatLng::new(latitude, longitude)?;
    if !(MIN_RANGE_METERS..=MAX_RANGE_METERS).contains(&range) {
        return Err("attendance range must be between 1 m and 100 km".to_string());
    }
    Ok(Some(AttendanceZone {
        center,
        range_meters: range,
    }))
}

pub fn format_coord(number: f64) -> String {
    let text = format!("{number:.8}");
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() || trimmed == "-" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn format_range(meters: f64) -> String {
    if meters.fract() == 0.0 {
        format!("{meters:.0}")
    } else {
        format_coord(meters)
    }
}

/// Configured attendance zone, if latitude, longitude, and range are all set.
pub async fn load_attendance_zone<C: ConnectionTrait>(
    db: &C,
) -> Result<Option<AttendanceZone>, String> {
    let prefs = load_hr_preferences(db).await?;
    match (prefs.attendance, prefs.attendance_range_meters) {
        (Some(center), Some(range_meters))
            if (MIN_RANGE_METERS..=MAX_RANGE_METERS).contains(&range_meters) =>
        {
            Ok(Some(AttendanceZone {
                center,
                range_meters,
            }))
        }
        _ => Ok(None),
    }
}

/// Store or clear the attendance zone on the singleton preferences row.
pub async fn save_attendance_zone(
    db: &DatabaseConnection,
    zone: Option<AttendanceZone>,
) -> Result<(), String> {
    let _ = load_hr_preferences(db).await?;
    match db.get_database_backend() {
        DatabaseBackend::Postgres => {
            use lariv_core::db::location::GeoPoint;
            let model = pg::Entity::find_by_id(1)
                .one(db)
                .await
                .map_err(|err| err.to_string())?
                .ok_or_else(|| "preferences missing".to_string())?;
            let mut active: pg::ActiveModel = model.into();
            active.attendance_position = Set(zone.map(|zone| GeoPoint::from(zone.center)));
            active.attendance_range_meters = Set(zone.map(|zone| zone.range_meters));
            active.updated_at = Set(Some(Utc::now()));
            active.update(db).await.map_err(|err| err.to_string())?;
        }
        _ => {
            let model = lite::Entity::find_by_id(1)
                .one(db)
                .await
                .map_err(|err| err.to_string())?
                .ok_or_else(|| "preferences missing".to_string())?;
            let mut active: lite::ActiveModel = model.into();
            active.attendance_latitude = Set(zone.map(|zone| zone.center.latitude));
            active.attendance_longitude = Set(zone.map(|zone| zone.center.longitude));
            active.attendance_range_meters = Set(zone.map(|zone| zone.range_meters));
            active.updated_at = Set(Some(Utc::now()));
            active.update(db).await.map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

#[derive(Clone)]
enum TextPref {
    EmployeeId(Option<String>),
    IdCard(Option<String>),
    SigningKey(Option<String>),
}

#[cfg(test)]
mod tests {
    use super::parse_attendance_zone;

    #[test]
    fn empty_zone_clears_and_partial_or_out_of_range_fails() {
        assert!(parse_attendance_zone("", "", "").unwrap().is_none());
        assert!(parse_attendance_zone("12.9", "", "").is_err());
        assert!(parse_attendance_zone("12.9", "77.5", "").is_err());
        assert!(parse_attendance_zone("91", "0", "10").is_err());
        assert!(parse_attendance_zone("12.9", "77.5", "0").is_err());
        assert!(parse_attendance_zone("12.9", "77.5", "100001").is_err());
        let zone = parse_attendance_zone("12.9716", "77.5946", "100")
            .unwrap()
            .unwrap();
        assert_eq!(zone.range_meters, 100.0);
        assert!((zone.center.latitude - 12.9716).abs() < 1e-9);
    }
}

async fn save_text_field(db: &DatabaseConnection, field: TextPref) -> Result<(), String> {
    let _ = load_hr_preferences(db).await?;
    match db.get_database_backend() {
        DatabaseBackend::Postgres => {
            let model = pg::Entity::find_by_id(1)
                .one(db)
                .await
                .map_err(|err| err.to_string())?
                .ok_or_else(|| "preferences missing".to_string())?;
            let mut active: pg::ActiveModel = model.into();
            match field.clone() {
                TextPref::EmployeeId(value) => active.employee_id_format = Set(value),
                TextPref::IdCard(value) => active.id_card_format = Set(value),
                TextPref::SigningKey(value) => active.id_card_signing_key = Set(value),
            }
            active.updated_at = Set(Some(Utc::now()));
            active.update(db).await.map_err(|err| err.to_string())?;
        }
        _ => {
            let model = lite::Entity::find_by_id(1)
                .one(db)
                .await
                .map_err(|err| err.to_string())?
                .ok_or_else(|| "preferences missing".to_string())?;
            let mut active: lite::ActiveModel = model.into();
            match field {
                TextPref::EmployeeId(value) => active.employee_id_format = Set(value),
                TextPref::IdCard(value) => active.id_card_format = Set(value),
                TextPref::SigningKey(value) => active.id_card_signing_key = Set(value),
            }
            active.updated_at = Set(Some(Utc::now()));
            active.update(db).await.map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}
