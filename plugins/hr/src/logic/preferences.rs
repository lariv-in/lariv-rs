//! Singleton HR preferences (`id = 1`).

use chrono::Utc;
use p256::ecdsa::SigningKey;
use p256::elliptic_curve::Generate;
use p256::pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, ExprTrait, QueryFilter, sea_query::Expr,
};

use crate::entities::preferences::{self, Entity as PreferencesEntity};

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
pub async fn load_hr_preferences<C: ConnectionTrait>(db: &C) -> Result<preferences::Model, String> {
    if let Some(prefs) = PreferencesEntity::find_by_id(1)
        .one(db)
        .await
        .map_err(|err| err.to_string())?
    {
        return Ok(prefs);
    }
    let now = Utc::now();
    preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        employee_id_format: Set(None),
        id_card_format: Set(None),
        id_card_signing_key: Set(None),
    }
    .insert(db)
    .await
    .map_err(|err| err.to_string())
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
    let mut active: preferences::ActiveModel = load_hr_preferences(db).await?.into();
    let format = format.trim();
    active.employee_id_format = Set(if format.is_empty() {
        None
    } else {
        Some(format.to_string())
    });
    active.updated_at = Set(Some(Utc::now()));
    active.update(db).await.map_err(|err| err.to_string())?;
    Ok(())
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
    let mut active: preferences::ActiveModel = load_hr_preferences(db).await?.into();
    let format = format.trim();
    active.id_card_format = Set(if format.is_empty() {
        None
    } else {
        Some(format.to_string())
    });
    active.updated_at = Set(Some(Utc::now()));
    active.update(db).await.map_err(|err| err.to_string())?;
    Ok(())
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
    let updated = PreferencesEntity::update_many()
        .col_expr(
            preferences::Column::IdCardSigningKey,
            Expr::value(pem.clone()),
        )
        .col_expr(preferences::Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(preferences::Column::Id.eq(1))
        .filter(
            preferences::Column::IdCardSigningKey
                .is_null()
                .or(preferences::Column::IdCardSigningKey.eq("")),
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
    let mut active: preferences::ActiveModel = load_hr_preferences(db).await?.into();
    active.id_card_signing_key = Set(Some(pem));
    active.updated_at = Set(Some(Utc::now()));
    active.update(db).await.map_err(|err| err.to_string())?;
    Ok(())
}
