use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::user_type::JoinedUserType;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "meets_joined_users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub conference_room_code: String,
    pub user_type: JoinedUserType,
    pub user_id: Option<i64>,
    pub anonymous_user_id: Option<i64>,
    pub joined_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::conference_room::Entity",
        from = "Column::ConferenceRoomCode",
        to = "super::conference_room::Column::Code",
        on_delete = "Cascade"
    )]
    ConferenceRoom,
    #[sea_orm(
        belongs_to = "crate::plugins::users::entities::user::Entity",
        from = "Column::UserId",
        to = "crate::plugins::users::entities::user::Column::Id",
        on_delete = "SetNull"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::anonymous_user::Entity",
        from = "Column::AnonymousUserId",
        to = "super::anonymous_user::Column::Id",
        on_delete = "SetNull"
    )]
    AnonymousUser,
}

impl Related<super::conference_room::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ConferenceRoom.def()
    }
}

impl Related<crate::plugins::users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::anonymous_user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AnonymousUser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type JoinedUser = Model;
