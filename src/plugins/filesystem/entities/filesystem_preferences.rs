use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plugins::filesystem::permissions::NodePermissions;

/// Singleton filesystem root access (`id = 1`).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "filesystem_preferences")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    #[sea_orm(indexed)]
    pub owner_id: Option<i64>,
    #[sea_orm(indexed)]
    pub role_id: Option<i64>,
    #[sea_orm(column_type = "Integer")]
    pub permissions: NodePermissions,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::plugins::users::entities::user::Entity",
        from = "Column::OwnerId",
        to = "crate::plugins::users::entities::user::Column::Id",
        on_delete = "SetNull",
        skip_fk
    )]
    /// FK is created in the migration. `skip_fk` keeps entity-derived test tables
    /// from requiring the users table.
    Owner,
    #[sea_orm(
        belongs_to = "crate::plugins::users::entities::role::Entity",
        from = "Column::RoleId",
        to = "crate::plugins::users::entities::role::Column::Id",
        on_delete = "SetNull",
        skip_fk
    )]
    /// FK is created in the migration. `skip_fk` keeps entity-derived test tables
    /// from requiring the roles table.
    Role,
}

impl Related<crate::plugins::users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Owner.def()
    }
}

impl Related<crate::plugins::users::entities::role::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Role.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type FilesystemPreferences = Model;
