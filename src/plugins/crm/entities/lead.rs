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
    pub company_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub source: LeadSource,
    pub notes: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::converted_lead::Entity")]
    ConvertedLead,
    #[sea_orm(has_one = "super::failed_lead::Entity")]
    FailedLead,
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

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn display_name(&self) -> String {
        let person = match (
            self.first_name.as_deref(),
            self.last_name.as_deref(),
        ) {
            (Some(f), Some(l)) if !f.is_empty() || !l.is_empty() => format!("{f} {l}").trim().to_string(),
            (Some(f), _) if !f.is_empty() => f.to_string(),
            (_, Some(l)) if !l.is_empty() => l.to_string(),
            _ => String::new(),
        };
        if !person.is_empty() {
            if let Some(c) = self.company_name.as_deref().filter(|s| !s.is_empty()) {
                return format!("{person} ({c})");
            }
            return person;
        }
        self.company_name
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("Lead #{}", self.id))
    }
}
