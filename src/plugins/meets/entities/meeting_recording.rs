use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "meets_meeting_recordings")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub conference_room_code: String,
    pub meeting_start_at: DateTime<Utc>,
    pub meeting_created_by_id: i64,
    pub video_recording_id: i64,
    pub transcript: String,
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
        from = "Column::MeetingCreatedById",
        to = "crate::plugins::users::entities::user::Column::Id",
        on_delete = "Restrict"
    )]
    MeetingCreatedBy,
    #[sea_orm(
        belongs_to = "crate::plugins::filesystem::entities::filesystem_node::Entity",
        from = "Column::VideoRecordingId",
        to = "crate::plugins::filesystem::entities::filesystem_node::Column::Id",
        on_delete = "Restrict"
    )]
    VideoRecording,
}

impl Related<super::conference_room::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ConferenceRoom.def()
    }
}

impl Related<crate::plugins::users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MeetingCreatedBy.def()
    }
}

impl Related<crate::plugins::filesystem::entities::filesystem_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VideoRecording.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type MeetingRecording = Model;
