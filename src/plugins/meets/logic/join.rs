use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::plugins::meets::entities::{
    ConferenceRoom,
    anonymous_user::{self, Entity as AnonymousUserEntity, Model as AnonymousUser},
    joined_user::{self, Entity as JoinedUserEntity, Model as JoinedUser},
    user_type::JoinedUserType,
};

pub fn membership_valid(row: &JoinedUser) -> bool {
    match row.user_type {
        JoinedUserType::Registered => row.user_id.is_some() && row.anonymous_user_id.is_none(),
        JoinedUserType::Anonymous => row.anonymous_user_id.is_some() && row.user_id.is_none(),
    }
}

pub async fn find_registered_join(
    db: &DatabaseConnection,
    room_code: &str,
    user_id: i64,
) -> Result<Option<JoinedUser>, String> {
    JoinedUserEntity::find()
        .filter(joined_user::Column::ConferenceRoomCode.eq(room_code))
        .filter(joined_user::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())
}

pub async fn find_anonymous_join(
    db: &DatabaseConnection,
    room_code: &str,
    anonymous_user_id: i64,
) -> Result<Option<JoinedUser>, String> {
    JoinedUserEntity::find()
        .filter(joined_user::Column::ConferenceRoomCode.eq(room_code))
        .filter(joined_user::Column::AnonymousUserId.eq(anonymous_user_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())
}

pub async fn join_registered(
    db: &DatabaseConnection,
    room: &ConferenceRoom,
    user_id: i64,
) -> Result<JoinedUser, String> {
    if let Some(existing) = find_registered_join(db, &room.code, user_id).await? {
        return Ok(existing);
    }
    if !room.joining_allowed && room.created_by_id != user_id {
        return Err("this meeting is locked".into());
    }
    let am = joined_user::ActiveModel {
        conference_room_code: Set(room.code.clone()),
        user_type: Set(JoinedUserType::Registered),
        user_id: Set(Some(user_id)),
        anonymous_user_id: Set(None),
        joined_at: Set(Utc::now()),
        ..Default::default()
    };
    am.insert(db).await.map_err(|e| e.to_string())
}

pub async fn get_anonymous_user(
    db: &DatabaseConnection,
    id: i64,
) -> Result<Option<AnonymousUser>, String> {
    AnonymousUserEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())
}

pub async fn join_anonymous(
    db: &DatabaseConnection,
    room: &ConferenceRoom,
    name: &str,
    email: &str,
    existing_anon_id: Option<i64>,
) -> Result<(AnonymousUser, JoinedUser), String> {
    if !room.anonymous_allowed {
        return Err("anonymous guests are not allowed".into());
    }
    if let Some(aid) = existing_anon_id
        && let Some(existing) = find_anonymous_join(db, &room.code, aid).await?
    {
        let anon = get_anonymous_user(db, aid)
            .await?
            .ok_or_else(|| "anonymous user missing".to_string())?;
        return Ok((anon, existing));
    }
    if !room.joining_allowed {
        return Err("this meeting is locked".into());
    }
    let name = name.trim();
    let email = email.trim();
    if name.is_empty() {
        return Err("name is required".into());
    }
    if email.is_empty() {
        return Err("email is required".into());
    }
    let anon = if let Some(aid) = existing_anon_id {
        match get_anonymous_user(db, aid).await? {
            Some(row) => row,
            None => create_anonymous(db, name, email).await?,
        }
    } else {
        create_anonymous(db, name, email).await?
    };
    let join = joined_user::ActiveModel {
        conference_room_code: Set(room.code.clone()),
        user_type: Set(JoinedUserType::Anonymous),
        user_id: Set(None),
        anonymous_user_id: Set(Some(anon.id)),
        joined_at: Set(Utc::now()),
        ..Default::default()
    };
    let joined = join.insert(db).await.map_err(|e| e.to_string())?;
    Ok((anon, joined))
}

async fn create_anonymous(
    db: &DatabaseConnection,
    name: &str,
    email: &str,
) -> Result<AnonymousUser, String> {
    let am = anonymous_user::ActiveModel {
        name: Set(name.to_string()),
        email: Set(email.to_string()),
        ..Default::default()
    };
    am.insert(db).await.map_err(|e| e.to_string())
}

pub async fn list_joined(
    db: &DatabaseConnection,
    room_code: &str,
) -> Result<Vec<JoinedUser>, String> {
    JoinedUserEntity::find()
        .filter(joined_user::Column::ConferenceRoomCode.eq(room_code))
        .all(db)
        .await
        .map_err(|e| e.to_string())
}
