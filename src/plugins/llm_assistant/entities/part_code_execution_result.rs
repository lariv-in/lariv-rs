use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_assistant_session_message_code_execution_results")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub llm_assistant_session_message_part_id: i64,
    pub outcome: String,
    pub output: Option<String>,
    pub executable_code_id: Option<String>,
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
