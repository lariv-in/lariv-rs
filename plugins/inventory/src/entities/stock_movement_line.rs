use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_stock_movement_lines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub stock_id: i64,
    pub stock_movement_id: i64,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub qty: Decimal,
    pub qty_unit: String,
    /// `lariv_formula::VariableType::as_str()`. Must match the stock.
    pub qty_type: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::stock::Entity",
        from = "Column::StockId",
        to = "super::stock::Column::Id",
        on_delete = "Restrict"
    )]
    Stock,
    #[sea_orm(
        belongs_to = "super::stock_movement::Entity",
        from = "Column::StockMovementId",
        to = "super::stock_movement::Column::Id",
        on_delete = "Cascade"
    )]
    Movement,
}

impl Related<super::stock::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Stock.def()
    }
}

impl Related<super::stock_movement::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Movement.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
