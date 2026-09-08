use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_assistant_session_vnode_reads")]
/// Last text-read snapshot of a VNode in a chat session (gates `edit_vnode`).
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    #[sea_orm(unique_key = "session_vnode")]
    pub session_id: i64,
    #[sea_orm(unique_key = "session_vnode")]
    pub vnode_id: i64,
    pub read_at: DateTime<Utc>,
    pub vnode_updated_at: Option<DateTime<Utc>>,
    pub content_sha256: Vec<u8>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::session::Entity",
        from = "Column::SessionId",
        to = "super::session::Column::Id",
        on_delete = "Cascade"
    )]
    Session,
    #[sea_orm(
        belongs_to = "crate::plugins::filesystem::entities::filesystem_node::Entity",
        from = "Column::VnodeId",
        to = "crate::plugins::filesystem::entities::filesystem_node::Column::Id",
        on_delete = "Cascade"
    )]
    VNode,
}

impl Related<super::session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Session.def()
    }
}

impl Related<crate::plugins::filesystem::entities::filesystem_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VNode.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type SessionVnodeRead = Model;
