//! Singleton Documents preferences (`id = 1`).

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use super::entities::document_preferences::{
    self, Entity as PrefsEntity, Model as DocumentPreferences,
};

/// Common name used until preferences are saved.
pub const DEFAULT_AUTHORITY_NAME: &str = "Lariv";

/// One year, matching [`crate::duration`] (`365` days).
pub const DEFAULT_VALIDITY_NS: i64 = 365 * 24 * 60 * 60 * 1_000_000_000;

/// X.520 common name upper bound.
pub const MAX_AUTHORITY_NAME_LEN: usize = 64;

/// Load the singleton row, creating the defaults when it is missing.
pub async fn load_preferences(
    db: &DatabaseConnection,
) -> Result<DocumentPreferences, sea_orm::DbErr> {
    if let Some(prefs) = PrefsEntity::find_by_id(1).one(db).await? {
        return Ok(prefs);
    }
    let now = Utc::now();
    let model = document_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        signing_authority_name: Set(DEFAULT_AUTHORITY_NAME.to_string()),
        validity_duration: Set(DEFAULT_VALIDITY_NS),
    };
    Ok(model.insert(db).await?)
}

/// Replace the signing fields on the singleton row.
pub async fn save_preferences(
    db: &DatabaseConnection,
    signing_authority_name: String,
    validity_duration: i64,
) -> Result<DocumentPreferences, sea_orm::DbErr> {
    let mut row: document_preferences::ActiveModel = load_preferences(db).await?.into();
    row.signing_authority_name = Set(signing_authority_name);
    row.validity_duration = Set(validity_duration);
    row.updated_at = Set(Some(Utc::now()));
    Ok(row.update(db).await?)
}

/// Reject a blank or over-long signing authority name.
pub fn validate_authority_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Signing authority name is required".to_string());
    }
    if name.chars().count() > MAX_AUTHORITY_NAME_LEN {
        return Err(format!(
            "Signing authority name must be at most {MAX_AUTHORITY_NAME_LEN} characters"
        ));
    }
    Ok(name)
}

/// Parse a duration field into nanoseconds. Signing needs at least one second.
pub fn parse_validity(value: &str) -> Result<i64, String> {
    let nanos = crate::duration::parse_duration(value)?;
    if nanos < 1_000_000_000 {
        return Err("Validity duration must be at least 1 second".to_string());
    }
    Ok(nanos)
}
