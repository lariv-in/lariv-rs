use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "crm_contacts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub company_id: i64,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub is_primary: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::plugins::crm::entities::company::Entity",
        from = "Column::CompanyId",
        to = "crate::plugins::crm::entities::company::Column::Id"
    )]
    Company,
    #[sea_orm(has_many = "crate::plugins::crm::entities::lead::Entity")]
    Lead,
}

impl Related<crate::plugins::crm::entities::company::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Company.def()
    }
}

impl Related<crate::plugins::crm::entities::lead::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lead.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn display_name(&self) -> String {
        self.name.clone()
    }
}
