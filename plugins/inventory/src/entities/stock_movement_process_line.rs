use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const PROCESS_INPUT: &str = "input";
pub const PROCESS_OUTPUT: &str = "output";

/// Which side of a process document a line belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum ProcessLineSide {
    #[sea_orm(string_value = "input")]
    Input,
    #[sea_orm(string_value = "output")]
    Output,
}

impl ProcessLineSide {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Input => PROCESS_INPUT,
            Self::Output => PROCESS_OUTPUT,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Input => "Process input",
            Self::Output => "Process output",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            PROCESS_INPUT => Some(Self::Input),
            PROCESS_OUTPUT => Some(Self::Output),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_stock_movement_process_lines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub stock_id: i64,
    pub movement_id: i64,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub qty: Decimal,
    pub qty_unit: String,
    /// `lariv_formula::VariableType::as_str()`. Must match the stock.
    pub qty_type: String,
    pub side: ProcessLineSide,
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
        belongs_to = "super::stock_movement_process::Entity",
        from = "Column::MovementId",
        to = "super::stock_movement_process::Column::Id",
        on_delete = "Cascade"
    )]
    Movement,
}

impl Related<super::stock::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Stock.def()
    }
}

impl Related<super::stock_movement_process::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Movement.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
