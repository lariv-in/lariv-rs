use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "blog_tags")]
/// SeaORM model row.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    /// Hierarchical tag path.
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::blog_tag_link::Entity")]
    BlogLinks,
}

impl Related<super::blog_tag_link::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BlogLinks.def()
    }
}

impl Related<super::blog::Entity> for Entity {
    fn to() -> RelationDef {
        super::blog_tag_link::Relation::Blog.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::blog_tag_link::Relation::Tag.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type BlogTag = Model;
