use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "crm_lead_tags")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    #[sea_orm(unique)]
    pub name: String,
    pub color: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::lead_tag_link::Entity")]
    LeadLinks,
}

impl Related<super::lead_tag_link::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LeadLinks.def()
    }
}

impl Related<super::lead::Entity> for Entity {
    fn to() -> RelationDef {
        super::lead_tag_link::Relation::Lead.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::lead_tag_link::Relation::Tag.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
