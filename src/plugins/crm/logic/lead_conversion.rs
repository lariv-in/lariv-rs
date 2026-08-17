use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, QueryFilter};

use crate::plugins::crm::entities::{
    company::Entity as CompanyEntity, contact::Entity as ContactEntity, converted_lead,
    lead::Entity as LeadEntity,
};
use crate::plugins::crm::logic::lead::err_if_lead_sealed;
use crate::plugins::crm::logic::lead_timeline::append_lead_timeline;
use crate::plugins::crm::scope::{find_active_lead, sql_lead_active};
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

pub async fn lead_is_active(db: &DatabaseConnection, lead_id: i64) -> bool {
    LeadEntity::find_by_id(lead_id)
        .filter(sql_lead_active())
        .one(db)
        .await
        .ok()
        .flatten()
        .is_some()
}
