use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "crm_converted_leads")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub lead_id: i64,
    pub converted_at: DateTime<Utc>,
    pub customer_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::lead::Entity",
        from = "Column::LeadId",
        to = "super::lead::Column::Id"
    )]
    Lead,
    #[sea_orm(
        belongs_to = "lariv_plugin_customer::entities::customer::Entity",
        from = "Column::CustomerId",
        to = "lariv_plugin_customer::entities::customer::Column::Id"
    )]
    Customer,
}

impl Related<super::lead::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lead.def()
    }
}

impl Related<lariv_plugin_customer::entities::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
