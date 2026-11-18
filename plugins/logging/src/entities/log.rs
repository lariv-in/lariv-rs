use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

/// Whether the audited statement inserted, updated, or deleted the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum LogKind {
    #[sea_orm(string_value = "Create")]
    Create,
    #[sea_orm(string_value = "Update")]
    Update,
    #[sea_orm(string_value = "Delete")]
    Delete,
}

impl LogKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Create => "Create",
            Self::Update => "Update",
            Self::Delete => "Delete",
        }
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "logs")]
/// One audited insert, update, or delete.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub table_name: String,
    pub row_id: i64,
    pub log_kind: LogKind,
    pub user_id: Option<i64>,
    /// When set, the change happened under this company session. Null means
    /// unassigned (visible to superusers under optional-company RLS).
    pub company_id: Option<i64>,
    #[sea_orm(column_type = "JsonBinary")]
    pub description: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "lariv_plugin_users::entities::user::Entity",
        from = "Column::UserId",
        to = "lariv_plugin_users::entities::user::Column::Id"
    )]
    User,
}

impl Related<lariv_plugin_users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type Log = Model;
