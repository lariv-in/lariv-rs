use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_assistant_session_message_inline_data")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub llm_assistant_session_message_part_id: i64,
    pub mime_type: String,
    pub data: Vec<u8>,
    pub display_name: Option<String>,
    /// Filesystem VNode this attachment was loaded from, when known.
    pub vnode_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::session_message_part::Entity",
        from = "Column::LlmAssistantSessionMessagePartId",
        to = "super::session_message_part::Column::Id",
        on_delete = "Cascade"
    )]
    Part,
}

impl ActiveModelBehavior for ActiveModel {}
