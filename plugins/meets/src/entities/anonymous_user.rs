use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "meets_anonymous_users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub email: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::joined_user::Entity")]
    JoinedUsers,
}

impl Related<super::joined_user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JoinedUsers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type AnonymousUser = Model;
