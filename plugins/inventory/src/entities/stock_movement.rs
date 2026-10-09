use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::movement_type::MovementType;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_stock_movements")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub datetime: DateTime<Utc>,
    pub movement_type: MovementType,
    pub number: String,
    pub bill_to_individual: bool,
    pub customer_individual: Option<i64>,
    pub customer_company: Option<i64>,
    pub vehicle_type: Option<String>,
    pub vehicle_number: Option<String>,
    pub eway_bill: Option<String>,
    pub driver_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::stock_movement_line::Entity")]
    Lines,
}

impl Related<super::stock_movement_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lines.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
