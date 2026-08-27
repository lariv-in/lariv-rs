use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::plugins::crm::entities::{
    company::Entity as CompanyEntity, contact::Entity as ContactEntity, converted_lead,
    converted_lead::Entity as ConvertedLeadEntity, lead::Entity as LeadEntity,
};
use crate::plugins::crm::logic::lead::err_if_lead_sealed;
use crate::plugins::crm::logic::lead_timeline::append_lead_timeline;
use crate::plugins::crm::scope::{find_active_lead, find_converted_lead_scoped, sql_lead_active};
use crate::plugins::users::state::AuthContext;

pub struct ConvertLeadResult {
    pub converted_id: i64,
    pub company_id: i64,
    pub contact_id: i64,
}

pub async fn convert_lead(
    db: &DatabaseConnection,
    lead_id: i64,
    auth: &AuthContext,
) -> Result<ConvertLeadResult, String> {
    let lead = find_active_lead(db, lead_id, auth)
        .await
        .ok_or_else(|| "lead not found or not active".to_string())?;
    err_if_lead_sealed(db, lead_id).await?;

    let contact_row = ContactEntity::find_by_id(lead.contact_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "contact not found".to_string())?;

    let company_row = CompanyEntity::find_by_id(contact_row.company_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "company not found".to_string())?;

    let now = Utc::now();
    let converted = converted_lead::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        lead_id: Set(lead.id),
        converted_at: Set(now),
        company_id: Set(company_row.id),
        contact_id: Set(contact_row.id),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;

    append_lead_timeline(db, lead.id, "Lead converted").await?;

    Ok(ConvertLeadResult {
        converted_id: converted.id,
        company_id: company_row.id,
        contact_id: contact_row.id,
    })
}

/// Remove the converted record so the lead is active again.
pub async fn unconvert_lead(
    db: &DatabaseConnection,
    converted_id: i64,
    auth: &AuthContext,
) -> Result<i64, String> {
    let converted = find_converted_lead_scoped(db, converted_id, auth)
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
    crate::web::opt_or_log(
        LeadEntity::find_by_id(lead_id)
            .filter(sql_lead_active())
            .one(db)
            .await,
        "find by id",
    )
    .is_some()
}
