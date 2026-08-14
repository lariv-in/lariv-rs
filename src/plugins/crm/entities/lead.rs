use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plugins::crm::lead_source::LeadSource;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "crm_leads")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub contact_id: i64,
    pub source: Option<LeadSource>,
    pub notes: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::contact::Entity",
        from = "Column::ContactId",
        to = "super::contact::Column::Id"
    )]
    Contact,
    #[sea_orm(has_one = "super::converted_lead::Entity")]
    ConvertedLead,
    #[sea_orm(has_one = "super::failed_lead::Entity")]
    FailedLead,
    #[sea_orm(has_many = "super::lead_update::Entity")]
    LeadUpdate,
    #[sea_orm(has_many = "super::lead_tag_link::Entity")]
    TagLinks,
}

impl Related<super::contact::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Contact.def()
    }
}

impl Related<super::converted_lead::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ConvertedLead.def()
    }
}

impl Related<super::failed_lead::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FailedLead.def()
    }
}

impl Related<super::lead_update::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LeadUpdate.def()
    }
}

impl Related<super::lead_tag_link::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TagLinks.def()
    }
}

impl Related<super::lead_tag::Entity> for Entity {
    fn to() -> RelationDef {
        super::lead_tag_link::Relation::Tag.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::lead_tag_link::Relation::Lead.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
