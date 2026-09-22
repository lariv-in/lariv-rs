use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};

use crate::plugins::meets::entities::{
    ConferenceRoom,
    conference_room::{self, Entity as ConferenceRoomEntity},
};
use crate::plugins::meets::logic::codes::generate_room_code;

pub struct CreateRoomInput {
    pub created_by_id: i64,
    pub anonymous_allowed: bool,
    pub joining_allowed: bool,
    pub scheduled_start_at: Option<DateTime<Utc>>,
}

pub async fn create_room(
    db: &DatabaseConnection,
    code_len: usize,
    input: CreateRoomInput,
) -> Result<ConferenceRoom, String> {
    let now = Utc::now();
    for _ in 0..8 {
        let code = generate_room_code(code_len);
        let am = conference_room::ActiveModel {
            code: Set(code),
            created_at: Set(now),
            created_by_id: Set(input.created_by_id),
            anonymous_allowed: Set(input.anonymous_allowed),
            joining_allowed: Set(input.joining_allowed),
            scheduled_start_at: Set(input.scheduled_start_at),
            started_at: Set(None),
            ended_at: Set(None),
            ..Default::default()
        };
        match am.insert(db).await {
            Ok(row) => return Ok(row),
            Err(e) => {
                let msg = e.to_string();
                if msg.to_ascii_lowercase().contains("unique")
                    || msg.to_ascii_lowercase().contains("constraint")
                {
                    continue;
                }
                return Err(msg);
            }
        }
    }
    Err("could not allocate a unique room code".into())
}

pub async fn find_room(
    db: &DatabaseConnection,
    code: &str,
) -> Result<Option<ConferenceRoom>, String> {
    ConferenceRoomEntity::find_by_id(code.to_string())
        .one(db)
        .await
        .map_err(|e| e.to_string())
}

pub async fn list_rooms(
    db: &DatabaseConnection,
    created_by_id: Option<i64>,
) -> Result<Vec<ConferenceRoom>, String> {
    let mut q = ConferenceRoomEntity::find().order_by_desc(conference_room::Column::CreatedAt);
    if let Some(uid) = created_by_id {
        q = q.filter(conference_room::Column::CreatedById.eq(uid));
    }
    q.all(db).await.map_err(|e| e.to_string())
}

pub fn room_is_live(room: &ConferenceRoom) -> bool {
    room.started_at.is_some() && room.ended_at.is_none()
}

pub async fn start_room(
    db: &DatabaseConnection,
    room: ConferenceRoom,
) -> Result<ConferenceRoom, String> {
    let now = Utc::now();
    let started = match room.scheduled_start_at {
        Some(t) if t <= now => t,
        _ => now,
    };
    let mut am: conference_room::ActiveModel = room.into();
    am.started_at = Set(Some(started));
    am.ended_at = Set(None);
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn stop_room(
    db: &DatabaseConnection,
    room: ConferenceRoom,
) -> Result<ConferenceRoom, String> {
    let mut am: conference_room::ActiveModel = room.into();
    am.ended_at = Set(Some(Utc::now()));
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn set_joining_allowed(
    db: &DatabaseConnection,
    room: ConferenceRoom,
    joining_allowed: bool,
) -> Result<ConferenceRoom, String> {
    let mut am: conference_room::ActiveModel = room.into();
    am.joining_allowed = Set(joining_allowed);
    am.update(db).await.map_err(|e| e.to_string())
}

pub struct UpdateRoomInput {
    pub anonymous_allowed: bool,
    pub joining_allowed: bool,
    pub scheduled_start_at: Option<DateTime<Utc>>,
}

pub async fn update_room(
    db: &DatabaseConnection,
    room: ConferenceRoom,
    input: UpdateRoomInput,
) -> Result<ConferenceRoom, String> {
    let mut am: conference_room::ActiveModel = room.into();
    am.anonymous_allowed = Set(input.anonymous_allowed);
    am.joining_allowed = Set(input.joining_allowed);
    am.scheduled_start_at = Set(input.scheduled_start_at);
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn delete_room(db: &DatabaseConnection, code: &str) -> Result<(), String> {
    ConferenceRoomEntity::delete_by_id(code.to_string())
        .exec(db)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}
