use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// A customer is one billed party: either a company or an individual contact.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "crm_customers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    /// `true` means [`Self::individual_id`] is set. `false` means [`Self::company_id`] is set.
    pub is_individual: bool,
    pub company_id: Option<i64>,
    pub individual_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "lariv_plugin_contacts::entities::company::Entity",
        from = "Column::CompanyId",
        to = "lariv_plugin_contacts::entities::company::Column::Id"
    )]
    Company,
    #[sea_orm(
        belongs_to = "lariv_plugin_contacts::entities::contact::Entity",
        from = "Column::IndividualId",
        to = "lariv_plugin_contacts::entities::contact::Column::Id"
    )]
    Individual,
}

impl Related<lariv_plugin_contacts::entities::company::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Company.def()
    }
}

impl Related<lariv_plugin_contacts::entities::contact::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Individual.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
