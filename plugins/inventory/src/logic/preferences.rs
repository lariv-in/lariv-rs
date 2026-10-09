//! Singleton inventory preferences (`id = 1`).

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use crate::entities::preferences::{self, Entity as PreferencesEntity};
use crate::forms::InventoryPreferencesForm;

pub const PREFERENCES_ID: i64 = 1;

pub async fn load_preferences(db: &DatabaseConnection) -> Result<preferences::Model, String> {
    if let Some(row) = PreferencesEntity::find_by_id(PREFERENCES_ID)
        .one(db)
        .await
        .map_err(|err| err.to_string())?
    {
        return Ok(row);
    }
    let now = Utc::now();
    preferences::ActiveModel {
        id: Set(PREFERENCES_ID),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        movement_number_format: Set(Some(
            crate::logic::number::DEFAULT_MOVEMENT_NUMBER_FORMAT.into(),
        )),
        company_name: Set(None),
        company_address: Set(None),
        company_phone: Set(None),
        company_email: Set(None),
        company_gstin: Set(None),
        terms_and_conditions: Set(None),
        logo_vnode_id: Set(None),
        signature_vnode_id: Set(None),
        movement_in_template: Set(None),
        movement_out_template: Set(None),
    }
    .insert(db)
    .await
    .map_err(|err| err.to_string())
}

pub async fn save_preferences(
    db: &DatabaseConnection,
    form: &InventoryPreferencesForm,
) -> Result<(), String> {
    let existing = load_preferences(db).await?;
    let now = Utc::now();
    let mut am: preferences::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.movement_number_format = Set(blank_to_none(&form.movement_number_format));
    am.company_name = Set(blank_to_none(&form.company_name));
    am.company_address = Set(blank_to_none(&form.company_address));
    am.company_phone = Set(blank_to_none(&form.company_phone));
    am.company_email = Set(blank_to_none(&form.company_email));
    am.company_gstin = Set(blank_to_none(&form.company_gstin));
    am.terms_and_conditions = Set(blank_to_none(&form.terms_and_conditions));
    am.logo_vnode_id = Set(parse_optional_id(&form.logo_vnode_id));
    am.signature_vnode_id = Set(parse_optional_id(&form.signature_vnode_id));
    am.movement_in_template = Set(blank_to_none(&form.movement_in_template));
    am.movement_out_template = Set(blank_to_none(&form.movement_out_template));
    am.update(db).await.map_err(|err| err.to_string())?;
    Ok(())
}

pub fn blank_to_none(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn parse_optional_id(raw: &str) -> Option<i64> {
    let id = raw.trim().parse::<i64>().unwrap_or(0);
    if id > 0 { Some(id) } else { None }
}

pub fn id_value(id: Option<i64>) -> String {
    id.filter(|id| *id > 0)
        .map(|id| id.to_string())
        .unwrap_or_default()
}
