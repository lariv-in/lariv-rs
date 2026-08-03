use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_assistant_session_message_tool_responses")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub llm_assistant_session_message_part_id: i64,
    pub tool_call_id: Option<String>,
    pub tool_type: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub response: Option<Json>,
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
