use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_assistant_sessions")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub title: String,
    #[sea_orm(indexed)]
    pub user_id: i64,
    /// When set, completed assistant replies are emailed to this address.
    pub reply_email: Option<String>,
    /// Inbound email `Message-ID` for SMTP threading.
    pub email_message_id: Option<String>,
    /// Inbound email `References` header chain.
    pub email_references: Option<String>,
    /// Tokens occupying the model context after the last generate (`usageMetadata.totalTokenCount`).
    #[sea_orm(default_value = 0)]
    pub context_tokens: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::plugins::users::entities::user::Entity",
        from = "Column::UserId",
        to = "crate::plugins::users::entities::user::Column::Id",
        on_delete = "Cascade"
    )]
    User,
    #[sea_orm(has_many = "super::session_message::Entity")]
    Messages,
    #[sea_orm(has_many = "super::session_compaction::Entity")]
    Compactions,
}

impl Related<crate::plugins::users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::session_message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Messages.def()
    }
}

impl Related<super::session_compaction::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Compactions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type Session = Model;
