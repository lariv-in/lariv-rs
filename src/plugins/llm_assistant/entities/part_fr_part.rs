use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_assistant_session_message_function_response_parts")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub llm_assistant_session_message_function_response_id: i64,
    pub kind: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::part_function_response::Entity",
        from = "Column::LlmAssistantSessionMessageFunctionResponseId",
        to = "super::part_function_response::Column::Id",
        on_delete = "Cascade"
    )]
    FunctionResponse,
}

impl Related<super::part_function_response::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FunctionResponse.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
