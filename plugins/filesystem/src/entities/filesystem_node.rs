use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::permissions::NodePermissions;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "filesystem_nodes")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    pub is_directory: bool,
    #[sea_orm(column_type = "Text", nullable)]
    pub file_path: Option<String>,
    #[sea_orm(indexed)]
    pub parent_id: Option<i64>,
    #[sea_orm(indexed)]
    pub owner_id: Option<i64>,
    #[sea_orm(column_type = "Text", nullable)]
    pub role: Option<String>,
    #[sea_orm(column_type = "Integer")]
    pub permissions: NodePermissions,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ParentId",
        to = "Column::Id",
        on_delete = "Cascade"
    )]
    Parent,
    #[sea_orm(
        belongs_to = "lariv_plugin_users::entities::user::Entity",
        from = "Column::OwnerId",
        to = "lariv_plugin_users::entities::user::Column::Id",
        on_delete = "SetNull",
        skip_fk
    )]
    /// FK is created in the migration. `skip_fk` keeps entity-derived test tables
    /// from requiring the users table.
    Owner,
}

impl Related<lariv_plugin_users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Owner.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type VNode = Model;
