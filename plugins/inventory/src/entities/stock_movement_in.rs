use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_stock_movements_in")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub datetime: DateTime<Utc>,
    pub number: String,
    pub bill_to_individual: bool,
    pub customer_individual: Option<i64>,
    pub customer_company: Option<i64>,
    pub vehicle_type: Option<String>,
    pub vehicle_number: Option<String>,
    pub eway_bill: Option<String>,
    pub driver_id: Option<i64>,
    /// Uploaded delivery challan file.
    pub delivery_challan: Option<i64>,
    /// User who saved this movement. Existing rows may be empty.
    pub created_by_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::stock_movement_in_line::Entity")]
    Lines,
    #[sea_orm(
        belongs_to = "lariv_plugin_filesystem::entities::filesystem_node::Entity",
        from = "Column::DeliveryChallan",
        to = "lariv_plugin_filesystem::entities::filesystem_node::Column::Id",
        on_delete = "Restrict"
    )]
    DeliveryChallan,
    #[sea_orm(
        belongs_to = "lariv_plugin_users::entities::user::Entity",
        from = "Column::CreatedById",
        to = "lariv_plugin_users::entities::user::Column::Id",
        on_delete = "Restrict"
    )]
    CreatedBy,
}

impl Related<super::stock_movement_in_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lines.def()
    }
}

impl Related<lariv_plugin_filesystem::entities::filesystem_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DeliveryChallan.def()
    }
}

impl Related<lariv_plugin_users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreatedBy.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
