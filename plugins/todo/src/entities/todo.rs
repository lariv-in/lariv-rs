use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "todos")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    #[sea_orm(indexed)]
    pub user_id: i64,
    #[sea_orm(default_value = false)]
    pub checked: bool,
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub description: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "lariv_plugin_users::entities::user::Entity",
        from = "Column::UserId",
        to = "lariv_plugin_users::entities::user::Column::Id",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<lariv_plugin_users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type Todo = Model;
