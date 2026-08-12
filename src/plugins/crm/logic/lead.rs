use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait,
    QueryFilter,
};

use crate::plugins::crm::entities::{
    converted_lead::Entity as ConvertedLeadEntity,
    failed_lead::Entity as FailedLeadEntity,
    lead::{self, Entity as LeadEntity},
};
use crate::plugins::crm::lead_source::LeadSource;
use crate::plugins::crm::scope::sql_lead_active;

pub async fn err_if_lead_sealed<C: ConnectionTrait>(db: &C, lead_id: i64) -> Result<(), String> {
    if lead_id == 0 {
        return Ok(());
    }
    let converted = ConvertedLeadEntity::find()
        .filter(crate::plugins::crm::entities::converted_lead::Column::LeadId.eq(lead_id))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if converted > 0 {
        return Err("lead is converted and cannot be changed".to_string());
    }
    let failed = FailedLeadEntity::find()
        .filter(crate::plugins::crm::entities::failed_lead::Column::LeadId.eq(lead_id))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if failed > 0 {
        return Err("lead is failed and cannot be changed".to_string());
    }
    Ok(())
}

pub struct LeadInput {
    pub contact_id: i64,
    pub source: Option<LeadSource>,
    pub notes: Option<String>,
}

pub async fn create_lead<C: ConnectionTrait>(
    db: &C,
    input: LeadInput,
) -> Result<lead::Model, String> {
    if input.contact_id <= 0 {
        return Err("contact is required".to_string());
    }
    let now = Utc::now();
    let model = lead::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        contact_id: Set(input.contact_id),
        source: Set(input.source),
        notes: Set(input.notes),
    };
    model.insert(db).await.map_err(|e| e.to_string())
}

pub async fn update_lead<C: ConnectionTrait>(
    db: &C,
    lead_id: i64,
    input: LeadInput,
) -> Result<lead::Model, String> {
    if input.contact_id <= 0 {
        return Err("contact is required".to_string());
    }
    let existing = LeadEntity::find_by_id(lead_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "lead not found".to_string())?;
    let now = Utc::now();
    let mut am: lead::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.contact_id = Set(input.contact_id);
    am.source = Set(input.source);
    am.notes = Set(input.notes);
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn delete_lead<C: ConnectionTrait>(db: &C, lead_id: i64) -> Result<(), String> {
    err_if_lead_sealed(db, lead_id).await?;
    let existing = LeadEntity::find_by_id(lead_id)
        .filter(sql_lead_active())
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "lead not found or not active".to_string())?;
    LeadEntity::delete_by_id(existing.id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
