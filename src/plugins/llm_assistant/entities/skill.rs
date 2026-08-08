use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "skills")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    #[sea_orm(unique)]
    pub name: String,
    pub description: String,
    #[sea_orm(column_type = "Text")]
    pub content: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::skill_file_link::Entity")]
    FileLinks,
}

impl Related<super::skill_file_link::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FileLinks.def()
    }
}

impl Related<crate::plugins::filesystem::entities::filesystem_node::Entity> for Entity {
    fn to() -> RelationDef {
        super::skill_file_link::Relation::VNode.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::skill_file_link::Relation::Skill.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type Skill = Model;
