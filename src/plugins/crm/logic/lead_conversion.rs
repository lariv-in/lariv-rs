use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, QueryFilter,
    TransactionTrait,
};

use crate::plugins::crm::entities::{
    company::{self, Entity as CompanyEntity},
    contact::Entity as ContactEntity,
    converted_lead,
    lead::Entity as LeadEntity,
};
use crate::plugins::crm::logic::lead::err_if_lead_sealed;
use crate::plugins::crm::scope::{find_active_lead, sql_lead_active};
use crate::plugins::customer::customer_type::CustomerType;
use crate::plugins::customer::entities::customer;
use crate::plugins::users::state::AuthContext;

pub struct ConvertLeadResult {
    pub converted_id: i64,
    pub company_id: i64,
    pub contact_id: i64,
    pub customer_id: i64,
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

    let txn = db.begin().await.map_err(|e| e.to_string())?;

    let now = Utc::now();

    let company_row = CompanyEntity::find_by_id(contact_row.company_id)
        .one(&txn)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "company not found".to_string())?;

    let customer_id = if let Some(cid) = company_row.customer_id {
        cid
    } else {
        let customer_row = customer::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            customer_type: Set(CustomerType::Business),
            name: Set(company_row.name.clone()),
            address_line_1: Set(None),
            address_line_2: Set(None),
            city: Set(None),
            pincode: Set(None),
            state: Set(None),
            gstin: Set(None),
            pan: Set(None),
            phone: Set(contact_row.phone.clone()),
            email: Set(contact_row.email.clone()),
            website: Set(None),
        }
        .insert(&txn)
        .await
        .map_err(|e| e.to_string())?;

        let mut company_am: company::ActiveModel = company_row.clone().into();
        company_am.updated_at = Set(Some(now));
        company_am.customer_id = Set(Some(customer_row.id));
        company_am.update(&txn).await.map_err(|e| e.to_string())?;
        customer_row.id
    };

    let converted = converted_lead::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        lead_id: Set(lead.id),
        converted_at: Set(now),
        company_id: Set(company_row.id),
        contact_id: Set(contact_row.id),
        customer_id: Set(customer_id),
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;

    txn.commit().await.map_err(|e| e.to_string())?;

    Ok(ConvertLeadResult {
        converted_id: converted.id,
        company_id: company_row.id,
        contact_id: contact_row.id,
        customer_id,
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
