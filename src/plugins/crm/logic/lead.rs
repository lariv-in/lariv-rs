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
    pub company_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub source: LeadSource,
    pub notes: Option<String>,
}

pub async fn create_lead<C: ConnectionTrait>(
    db: &C,
    input: LeadInput,
) -> Result<lead::Model, String> {
    let now = Utc::now();
    let model = lead::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        company_name: Set(input.company_name),
        first_name: Set(input.first_name),
        last_name: Set(input.last_name),
        email: Set(input.email),
        phone: Set(input.phone),
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
    let existing = LeadEntity::find_by_id(lead_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "lead not found".to_string())?;
    let now = Utc::now();
    let mut am: lead::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.company_name = Set(input.company_name);
    am.first_name = Set(input.first_name);
    am.last_name = Set(input.last_name);
    am.email = Set(input.email);
    am.phone = Set(input.phone);
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
