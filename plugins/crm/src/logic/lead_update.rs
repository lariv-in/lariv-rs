//! Lead detail Updates panel rows.

use chrono::{DateTime, Utc};
use sea_orm::sea_query::{Condition, Expr, ExprTrait, Func};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

use crate::entities::lead_update::{self, Entity as LeadUpdateEntity};

pub async fn append_lead_update<C: ConnectionTrait>(
    db: &C,
    lead_id: i64,
    created_by_id: i64,
    description: &str,
) -> Result<(), String> {
    let now = Utc::now();
    lead_update::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        lead_id: Set(lead_id),
        created_by_id: Set(created_by_id),
        datetime: Set(now),
        description: Set(description.to_string()),
    }
    .insert(db)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub struct LeadUpdateWrite {
    pub lead_id: i64,
    pub created_by_id: i64,
    pub datetime: DateTime<Utc>,
    pub description: String,
}

pub async fn create_lead_update<C: ConnectionTrait>(
    db: &C,
    input: LeadUpdateWrite,
) -> Result<lead_update::Model, String> {
    require_description(&input.description)?;
    ensure_lead(db, input.lead_id).await?;
    let now = Utc::now();
    lead_update::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        lead_id: Set(input.lead_id),
        created_by_id: Set(input.created_by_id),
        datetime: Set(input.datetime),
        description: Set(input.description),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn update_lead_update<C: ConnectionTrait>(
    db: &C,
    update_id: i64,
    input: LeadUpdateWrite,
) -> Result<lead_update::Model, String> {
    require_description(&input.description)?;
    ensure_lead(db, input.lead_id).await?;
    let existing = get_lead_update(db, update_id).await?;
    let now = Utc::now();
    let mut am: lead_update::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.lead_id = Set(input.lead_id);
    am.created_by_id = Set(input.created_by_id);
    am.datetime = Set(input.datetime);
    am.description = Set(input.description);
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn get_lead_update<C: ConnectionTrait>(
    db: &C,
    update_id: i64,
) -> Result<lead_update::Model, String> {
    if update_id <= 0 {
        return Err("get_lead_update requires a positive lead update id".to_string());
    }
    LeadUpdateEntity::find_by_id(update_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("lead update {update_id} not found"))
}

pub async fn search_lead_updates<C: ConnectionTrait>(
    db: &C,
    lead_id: Option<i64>,
    query: &str,
    limit: u64,
) -> Result<Vec<lead_update::Model>, String> {
    let mut select = LeadUpdateEntity::find();
    if let Some(lead_id) = lead_id.filter(|id| *id > 0) {
        select = select.filter(lead_update::Column::LeadId.eq(lead_id));
    }
    let needle = query.trim();
    if !needle.is_empty() {
        let like = format!(
            "%{}%",
            needle
                .to_lowercase()
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        select = select.filter(
            Condition::all()
                .add(Func::lower(Expr::col(lead_update::Column::Description)).like(like)),
        );
    }
    select
        .order_by_desc(lead_update::Column::Id)
        .limit(limit)
        .all(db)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_lead_update<C: ConnectionTrait>(db: &C, update_id: i64) -> Result<(), String> {
    let existing = get_lead_update(db, update_id).await?;
    LeadUpdateEntity::delete_by_id(existing.id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn require_description(description: &str) -> Result<(), String> {
    if description.trim().is_empty() {
        Err("description is required".to_string())
    } else {
        Ok(())
    }
}

async fn ensure_lead<C: ConnectionTrait>(db: &C, lead_id: i64) -> Result<(), String> {
    if lead_id <= 0 {
        return Err("lead not found".to_string());
    }
    let found = crate::entities::lead::Entity::find_by_id(lead_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    if found.is_none() {
        return Err("lead not found".to_string());
    }
    Ok(())
}

/// `active`, `converted`, or `failed`.
pub async fn lead_status<C: ConnectionTrait>(db: &C, lead_id: i64) -> Result<&'static str, String> {
    ensure_lead(db, lead_id).await?;
    let converted = crate::entities::converted_lead::Entity::find()
        .filter(crate::entities::converted_lead::Column::LeadId.eq(lead_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    if converted.is_some() {
        return Ok("converted");
    }
    let failed = crate::entities::failed_lead::Entity::find()
        .filter(crate::entities::failed_lead::Column::LeadId.eq(lead_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    if failed.is_some() {
        Ok("failed")
    } else {
        Ok("active")
    }
}
