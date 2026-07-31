use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_assistant_session_message_parts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub kind: String,
    pub llm_assistant_session_message_id: i64,
    pub thought: bool,
    pub thought_signature: Option<Vec<u8>>,
    pub video_metadata_id: Option<i64>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub part_metadata: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::session_message::Entity",
        from = "Column::LlmAssistantSessionMessageId",
        to = "super::session_message::Column::Id",
        on_delete = "Cascade"
    )]
    Message,
    #[sea_orm(
        belongs_to = "super::video_metadata::Entity",
        from = "Column::VideoMetadataId",
        to = "super::video_metadata::Column::Id",
        on_delete = "SetNull"
    )]
    VideoMetadata,
}

impl Related<super::session_message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Message.def()
    }
}

impl Related<super::video_metadata::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VideoMetadata.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type SessionMessagePart = Model;
