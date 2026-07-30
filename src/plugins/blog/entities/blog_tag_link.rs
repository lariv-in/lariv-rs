use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p_blog_tags")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub blog_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub blog_tag_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::blog::Entity",
        from = "Column::BlogId",
        to = "super::blog::Column::Id",
        on_delete = "Cascade"
    )]
    Blog,
    #[sea_orm(
        belongs_to = "super::blog_tag::Entity",
        from = "Column::BlogTagId",
        to = "super::blog_tag::Column::Id",
        on_delete = "Cascade"
    )]
    Tag,
}

impl Related<super::blog::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Blog.def()
    }
}

impl Related<super::blog_tag::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tag.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type BlogTagLink = Model;
