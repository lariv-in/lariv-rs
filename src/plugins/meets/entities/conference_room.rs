use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "meets_conference_rooms")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub code: String,
    pub created_at: DateTime<Utc>,
    pub created_by_id: i64,
    pub anonymous_allowed: bool,
    pub joining_allowed: bool,
    pub start_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::plugins::users::entities::user::Entity",
        from = "Column::CreatedById",
        to = "crate::plugins::users::entities::user::Column::Id",
        on_delete = "Restrict"
    )]
    CreatedBy,
    #[sea_orm(has_many = "super::joined_user::Entity")]
    JoinedUsers,
    #[sea_orm(has_many = "super::meeting_recording::Entity")]
    Recordings,
}

impl Related<crate::plugins::users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreatedBy.def()
    }
}

impl Related<super::joined_user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JoinedUsers.def()
    }
}

impl Related<super::meeting_recording::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Recordings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type ConferenceRoom = Model;
