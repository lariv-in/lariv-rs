use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter,
};

use crate::plugins::crm::entities::{
    converted_lead::Entity as ConvertedLeadEntity,
    failed_lead::{self, Entity as FailedLeadEntity},
};
use crate::plugins::crm::logic::lead::err_if_lead_sealed;
use crate::plugins::crm::logic::lead_timeline::append_lead_timeline;
use crate::plugins::crm::scope::{find_active_lead, find_failed_lead_scoped, find_lead_scoped};
use crate::plugins::users::state::AuthContext;

pub async fn fail_lead(
    db: &DatabaseConnection,
    lead_id: i64,
    auth: &AuthContext,
    reason: Option<String>,
) -> Result<i64, String> {
    let lead = find_active_lead(db, lead_id, auth)
        .await
        .ok_or_else(|| "lead not found or not active".to_string())?;
    err_if_lead_sealed(db, lead_id).await?;

    let now = Utc::now();
    let row = failed_lead::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        lead_id: Set(lead.id),
        failed_at: Set(now),
        reason: Set(reason.filter(|s| !s.trim().is_empty())),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;

    let content = match row
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(reason) => format!("Lead marked failed: {reason}"),
        None => "Lead marked failed".to_string(),
    };
    append_lead_timeline(db, lead.id, content).await?;

    Ok(row.id)
}

pub async fn reactivate_lead(
    db: &DatabaseConnection,
    failed_id: i64,
    auth: &AuthContext,
) -> Result<i64, String> {
    let failed = find_failed_lead_scoped(db, failed_id, auth)
        .await
        .ok_or_else(|| "failed lead not found".to_string())?;

    let converted = ConvertedLeadEntity::find()
        .filter(crate::plugins::crm::entities::converted_lead::Column::LeadId.eq(failed.lead_id))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if converted > 0 {
        return Err("lead is converted and cannot be reactivated".to_string());
    }

    find_lead_scoped(db, failed.lead_id, auth)
        .await
        .ok_or_else(|| "lead not found".to_string())?;

    FailedLeadEntity::delete_by_id(failed.id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;

    append_lead_timeline(db, failed.lead_id, "Lead reactivated").await?;

    Ok(failed.lead_id)
}

pub async fn update_failed_reason(
    db: &DatabaseConnection,
    lead_id: i64,
    auth: &AuthContext,
    reason: Option<String>,
) -> Result<(), String> {
    let failed = FailedLeadEntity::find()
        .filter(failed_lead::Column::LeadId.eq(lead_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "failed lead not found".to_string())?;
    find_failed_lead_scoped(db, failed.id, auth)
        .await
        .ok_or_else(|| "failed lead not found".to_string())?;

    let mut am: failed_lead::ActiveModel = failed.into();
    am.reason = Set(reason.filter(|s| !s.trim().is_empty()));
    am.update(db).await.map_err(|e| e.to_string()).map(|_| ())
}
