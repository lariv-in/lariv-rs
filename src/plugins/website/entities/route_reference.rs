use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p_website_route_references")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub db_route_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub v_node_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::db_route::Entity",
        from = "Column::DbRouteId",
        to = "super::db_route::Column::Id",
        on_delete = "Cascade"
    )]
    DbRoute,
    #[sea_orm(
        belongs_to = "crate::plugins::filesystem::entities::filesystem_node::Entity",
        from = "Column::VNodeId",
        to = "crate::plugins::filesystem::entities::filesystem_node::Column::Id",
        on_delete = "Cascade"
    )]
    VNode,
}

impl Related<super::db_route::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DbRoute.def()
    }
}

impl Related<crate::plugins::filesystem::entities::filesystem_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VNode.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type RouteReference = Model;
