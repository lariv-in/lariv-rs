use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p_crm_lead_tags")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub lead_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub lead_tag_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::lead::Entity",
        from = "Column::LeadId",
        to = "super::lead::Column::Id",
        on_delete = "Cascade"
    )]
    Lead,
    #[sea_orm(
        belongs_to = "super::lead_tag::Entity",
        from = "Column::LeadTagId",
        to = "super::lead_tag::Column::Id",
        on_delete = "Cascade"
    )]
    Tag,
}

impl Related<super::lead::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lead.def()
    }
}

impl Related<super::lead_tag::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tag.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
