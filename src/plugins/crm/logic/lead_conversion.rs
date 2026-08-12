use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait,
};

use crate::plugins::crm::deal_stage::DealStage;
use crate::plugins::crm::entities::{
    company::{self, Entity as CompanyEntity},
    contact, converted_lead, deal,
    lead::Entity as LeadEntity,
};
use crate::plugins::crm::logic::lead::err_if_lead_sealed;
use crate::plugins::crm::scope::{find_active_lead, sql_lead_active};
use crate::plugins::customer::customer_type::CustomerType;
use crate::plugins::customer::entities::customer;
use crate::plugins::users::state::AuthContext;

#[derive(Debug, Clone)]
pub enum ConvertLeadDeal {
    None,
    Create {
        deal_name: Option<String>,
        deal_amount: Option<rust_decimal::Decimal>,
        deal_stage: DealStage,
    },
}

pub struct ConvertLeadInput {
    pub company_id: i64,
    pub deal: ConvertLeadDeal,
}

pub struct ConvertLeadResult {
    pub converted_id: i64,
    pub company_id: i64,
    pub contact_id: i64,
    pub customer_id: i64,
    pub deal_id: Option<i64>,
}

pub async fn convert_lead(
    db: &DatabaseConnection,
    lead_id: i64,
    auth: &AuthContext,
    input: ConvertLeadInput,
) -> Result<ConvertLeadResult, String> {
    let lead = find_active_lead(db, lead_id, auth)
        .await
        .ok_or_else(|| "lead not found or not active".to_string())?;
    err_if_lead_sealed(db, lead_id).await?;

    let txn = db.begin().await.map_err(|e| e.to_string())?;

    let now = Utc::now();

    let company_row = if input.company_id <= 0 {
        return Err("company is required".to_string());
    } else {
        CompanyEntity::find_by_id(input.company_id)
            .one(&txn)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "company not found".to_string())?
    };

    let first_name = lead
        .first_name
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Contact".to_string());
    let contact_row = contact::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        company_id: Set(company_row.id),
        first_name: Set(first_name),
        last_name: Set(lead.last_name.clone()),
        email: Set(lead.email.clone()),
        phone: Set(lead.phone.clone()),
        title: Set(None),
        is_primary: Set(true),
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;

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
            phone: Set(lead.phone.clone()),
            email: Set(lead.email.clone()),
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

    let deal_id = match input.deal {
        ConvertLeadDeal::None => None,
        ConvertLeadDeal::Create {
            deal_name,
            deal_amount,
            deal_stage,
        } => {
            let deal_name = deal_name
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("Opportunity — {}", company_row.name));
            let deal_row = deal::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                company_id: Set(company_row.id),
                primary_contact_id: Set(contact_row.id),
                name: Set(deal_name),
                amount: Set(deal_amount),
                stage: Set(deal_stage),
                expected_close_date: Set(None),
            }
            .insert(&txn)
            .await
            .map_err(|e| e.to_string())?;
            Some(deal_row.id)
        }
    };

    let converted = converted_lead::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        lead_id: Set(lead.id),
        converted_at: Set(now),
        company_id: Set(company_row.id),
        contact_id: Set(contact_row.id),
        customer_id: Set(customer_id),
        deal_id: Set(deal_id),
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
        deal_id,
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
