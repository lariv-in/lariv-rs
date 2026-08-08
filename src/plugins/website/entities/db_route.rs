use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "db_routes")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    #[sea_orm(unique)]
    pub path: String,
    pub page_id: i64,
    pub is_active: bool,
    /// GrapesJSThemes registry key; empty means no theme.
    #[sea_orm(column_type = "String(StringLen::N(128))")]
    pub theme: String,
    /// GrapesJS project JSON for re-editing.
    #[sea_orm(column_type = "Text", nullable)]
    pub grapes_project: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::plugins::filesystem::entities::filesystem_node::Entity",
        from = "Column::PageId",
        to = "crate::plugins::filesystem::entities::filesystem_node::Column::Id",
        on_delete = "Restrict"
    )]
    Page,
    #[sea_orm(has_many = "super::route_reference::Entity")]
    References,
}

impl Related<crate::plugins::filesystem::entities::filesystem_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Page.def()
    }
}

impl Related<super::route_reference::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::References.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type DbRoute = Model;
