use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_stocks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    pub company_id: i64,
    /// `lariv_formula::VariableType::as_str()`.
    pub qty_type: String,
    /// Display unit. Set for length (`mm`, `cm`, …) and weight (`mg`, `g`, `kg`, `t`, `lb`); empty otherwise.
    pub qty_unit: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "lariv_plugin_contacts::entities::company::Entity",
        from = "Column::CompanyId",
        to = "lariv_plugin_contacts::entities::company::Column::Id",
        on_delete = "Restrict"
    )]
    Company,
    #[sea_orm(has_many = "super::stock_movement_line::Entity")]
    Lines,
}

impl Related<lariv_plugin_contacts::entities::company::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Company.def()
    }
}

impl Related<super::stock_movement_line::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lines.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
