use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "blogs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub title: String,
    pub slug: String,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    pub created_by_id: i64,
    #[sea_orm(column_type = "Text")]
    pub content: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::plugins::users::entities::user::Entity",
        from = "Column::CreatedById",
        to = "crate::plugins::users::entities::user::Column::Id",
        on_delete = "Cascade"
    )]
    CreatedBy,
    #[sea_orm(has_many = "super::blog_tag_link::Entity")]
    TagLinks,
}

impl Related<crate::plugins::users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreatedBy.def()
    }
}

impl Related<super::blog_tag_link::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TagLinks.def()
    }
}

impl Related<super::blog_tag::Entity> for Entity {
    fn to() -> RelationDef {
        super::blog_tag_link::Relation::Tag.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::blog_tag_link::Relation::Blog.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type Blog = Model;
