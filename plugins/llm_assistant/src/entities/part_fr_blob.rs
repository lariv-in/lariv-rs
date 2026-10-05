use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_assistant_session_message_function_response_blobs")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub llm_assistant_session_message_function_response_part_id: i64,
    pub mime_type: String,
    pub data: Vec<u8>,
    pub display_name: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::part_fr_part::Entity",
        from = "Column::LlmAssistantSessionMessageFunctionResponsePartId",
        to = "super::part_fr_part::Column::Id",
        on_delete = "Cascade"
    )]
    FrPart,
}

impl ActiveModelBehavior for ActiveModel {}
