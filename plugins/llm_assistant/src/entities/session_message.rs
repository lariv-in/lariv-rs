use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_assistant_session_messages")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub llm_assistant_session_id: i64,
    pub role: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::session::Entity",
        from = "Column::LlmAssistantSessionId",
        to = "super::session::Column::Id",
        on_delete = "Cascade"
    )]
    Session,
    #[sea_orm(has_many = "super::session_message_part::Entity")]
    Parts,
}

impl Related<super::session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Session.def()
    }
}

impl Related<super::session_message_part::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Parts.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type SessionMessage = Model;
