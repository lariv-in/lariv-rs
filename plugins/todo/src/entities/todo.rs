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
    /// How many title splices have been applied. A splice is stored only when it names this revision.
    #[sea_orm(default_value = 0)]
    pub title_rev: i64,
    /// Client-generated id so a resent create does not insert a second row.
    #[sea_orm(unique, column_type = "Text")]
    pub client_id: Option<String>,
    /// Page this item belongs to. The page is visible only to its creator.
    #[sea_orm(indexed)]
    pub todo_page_id: i64,
    /// Position on the page. The checklist is sorted by this, then by id.
    #[sea_orm(column_name = "sort_order", default_value = 0)]
    pub order: i64,
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
    #[sea_orm(
        belongs_to = "super::todo_page::Entity",
        from = "Column::TodoPageId",
        to = "super::todo_page::Column::Id",
        on_delete = "Cascade"
    )]
    TodoPage,
}

impl Related<lariv_plugin_users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::todo_page::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TodoPage.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type Todo = Model;
