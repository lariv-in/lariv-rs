use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait};

use crate::plugins::crm::entities::lead_timeline;

pub async fn append_lead_timeline<C: ConnectionTrait>(
    db: &C,
    lead_id: i64,
    content: impl Into<String>,
) -> Result<lead_timeline::Model, String> {
    let now = Utc::now();
    lead_timeline::ActiveModel {
        id: Default::default(),
        created_at: Set(now),
        content: Set(content.into()),
        lead_id: Set(lead_id),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}
