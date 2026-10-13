use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Out movements that draw stock from a process movement's output.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_stock_movement_out_processes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub out_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub process_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::stock_movement_out::Entity",
        from = "Column::OutId",
        to = "super::stock_movement_out::Column::Id",
        on_delete = "Cascade"
    )]
    Out,
    #[sea_orm(
        belongs_to = "super::stock_movement_process::Entity",
        from = "Column::ProcessId",
        to = "super::stock_movement_process::Column::Id",
        on_delete = "Restrict"
    )]
    Process,
}

impl Related<super::stock_movement_out::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Out.def()
    }
}

impl Related<super::stock_movement_process::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Process.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
