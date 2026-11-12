use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::entities::{
    converted_lead, converted_lead::Entity as ConvertedLeadEntity, lead::Entity as LeadEntity,
};
use crate::logic::lead::err_if_lead_sealed;
use crate::logic::lead_timeline::append_lead_timeline;
use crate::scope::{find_active_lead, find_converted_lead_scoped, lead_active_condition};
use lariv_plugin_users::state::AuthContext;

pub struct ConvertLeadResult {
    pub converted_id: i64,
    pub customer_id: i64,
}

pub async fn convert_lead(
    db: &DatabaseConnection,
    lead_id: i64,
    _auth: &AuthContext,
) -> Result<ConvertLeadResult, String> {
    let lead = find_active_lead(db, lead_id)
        .await
        .ok_or_else(|| "lead not found or not active".to_string())?;
    err_if_lead_sealed(db, lead_id).await?;
    write_conversion(db, &lead).await
}

pub async fn convert_active_lead<C: sea_orm::ConnectionTrait>(
    db: &C,
    lead_id: i64,
) -> Result<ConvertLeadResult, String> {
    let lead = LeadEntity::find_by_id(lead_id)
        .filter(lead_active_condition())
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "lead not found or not active".to_string())?;
    write_conversion(db, &lead).await
}

async fn write_conversion<C: sea_orm::ConnectionTrait>(
    db: &C,
    lead: &crate::entities::lead::Model,
) -> Result<ConvertLeadResult, String> {
    if lead.customer_id <= 0 {
        return Err("customer is required to convert a lead".to_string());
    }

    let now = Utc::now();
    let converted = converted_lead::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        lead_id: Set(lead.id),
        converted_at: Set(now),
        customer_id: Set(lead.customer_id),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;

    append_lead_timeline(db, lead.id, "Lead converted").await?;

    Ok(ConvertLeadResult {
        converted_id: converted.id,
        customer_id: lead.customer_id,
    })
}

/// Remove the converted record so the lead is active again.
pub async fn unconvert_lead(
    db: &DatabaseConnection,
    converted_id: i64,
    _auth: &AuthContext,
) -> Result<i64, String> {
    let converted = find_converted_lead_scoped(db, converted_id)
        .await
        .ok_or_else(|| "converted lead not found".to_string())?;

    ConvertedLeadEntity::delete_by_id(converted.id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;

    append_lead_timeline(db, converted.lead_id, "Lead made active again").await?;

    Ok(converted.lead_id)
}

/// Delete any converted row for `lead_id` (no-op if none). Used before failing a converted lead.
pub async fn clear_converted_for_lead(
    db: &DatabaseConnection,
    lead_id: i64,
) -> Result<bool, String> {
    let Some(converted) = ConvertedLeadEntity::find()
        .filter(converted_lead::Column::LeadId.eq(lead_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(false);
    };
    ConvertedLeadEntity::delete_by_id(converted.id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
}

pub async fn lead_is_active(db: &DatabaseConnection, lead_id: i64) -> bool {
    lariv_core::web::opt_or_log(
        LeadEntity::find_by_id(lead_id)
            .filter(lead_active_condition())
            .one(db)
            .await,
        "find by id",
    )
    .is_some()
}
