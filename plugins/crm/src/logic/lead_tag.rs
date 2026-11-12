//! Lead tag writes shared by the HTTP handlers and assistant tools.

use sea_orm::sea_query::{Condition, Expr, ExprTrait, Func};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
};

use crate::entities::lead_tag::{self, Entity as LeadTagEntity};

pub const DEFAULT_TAG_COLOR: &str = "#6366f1";

/// `#rrggbb`, lowercased. `None` when `raw` is not a 6-digit hex color.
pub fn parsed_tag_color(raw: &str) -> Option<String> {
    let hex = raw.trim().strip_prefix('#').unwrap_or(raw.trim());
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(format!("#{}", hex.to_ascii_lowercase()))
    } else {
        None
    }
}

pub async fn create_lead_tag<C: ConnectionTrait>(
    db: &C,
    name: &str,
    color: &str,
) -> Result<lead_tag::Model, String> {
    let name = require_name(name)?;
    let now = chrono::Utc::now();
    lead_tag::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(name),
        color: Set(color.to_string()),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn update_lead_tag<C: ConnectionTrait>(
    db: &C,
    tag_id: i64,
    name: &str,
    color: &str,
) -> Result<lead_tag::Model, String> {
    let name = require_name(name)?;
    let existing = LeadTagEntity::find_by_id(tag_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("lead tag {tag_id} not found"))?;
    let now = chrono::Utc::now();
    let mut am: lead_tag::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(name);
    am.color = Set(color.to_string());
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn delete_lead_tag<C: ConnectionTrait>(db: &C, tag_id: i64) -> Result<(), String> {
    if tag_id <= 0 {
        return Err("delete_lead_tag requires a positive lead tag id".to_string());
    }
    let found = LeadTagEntity::find_by_id(tag_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    if found.is_none() {
        return Err(format!("lead tag {tag_id} not found"));
    }
    LeadTagEntity::delete_by_id(tag_id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_lead_tag<C: ConnectionTrait>(
    db: &C,
    tag_id: i64,
) -> Result<lead_tag::Model, String> {
    if tag_id <= 0 {
        return Err("get_lead_tag requires a positive lead tag id".to_string());
    }
    LeadTagEntity::find_by_id(tag_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("lead tag {tag_id} not found"))
}

pub async fn search_lead_tags<C: ConnectionTrait>(
    db: &C,
    query: &str,
    limit: u64,
) -> Result<Vec<lead_tag::Model>, String> {
    let mut select = LeadTagEntity::find();
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
            Condition::all().add(Func::lower(Expr::col(lead_tag::Column::Name)).like(like)),
        );
    }
    select
        .order_by_desc(lead_tag::Column::Id)
        .limit(limit)
        .all(db)
        .await
        .map_err(|e| e.to_string())
}

fn require_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        Err("name is required".to_string())
    } else {
        Ok(name.to_string())
    }
}
