use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    #[sea_orm(unique, column_type = "Text")]
    pub email: crate::plugins::users::null_text::NullText,
    #[sea_orm(unique, column_type = "Text")]
    pub phone: crate::plugins::users::phone::Phone,
    pub is_superuser: bool,
    pub role_id: i64,
    #[sea_orm(column_name = "password")]
    pub password_hash: Option<Vec<u8>>,
    pub password_salt: Option<Vec<u8>>,
    #[sea_orm(column_type = "Text")]
    pub timezone: crate::plugins::users::null_text::NullText,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::role::Entity",
        from = "Column::RoleId",
        to = "super::role::Column::Id"
    )]
    Role,
}

impl Related<super::role::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Role.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type User = Model;
