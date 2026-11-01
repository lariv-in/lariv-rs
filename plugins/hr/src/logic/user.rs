use chrono::Utc;
use rand::RngExt;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, EntityTrait,
};

use crate::roles;
use lariv_plugin_users::roles::Superuser;
use lariv_plugin_users::{entities::user, password};

use super::person::{PersonInput, normalized_person_input, validate_person_input};

use lariv_plugin_users::entities::user::Entity as UserEntity;
use sea_orm::{ColumnTrait, QueryFilter};
use std::collections::HashMap;

/// Display name for a users-table id (empty when `id` is not positive).
pub async fn user_display_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    lariv_core::web::opt_or_log(UserEntity::find_by_id(id).one(db).await, "find user by id")
        .map(|row| row.name)
        .unwrap_or_else(|| format!("User {id}"))
}

/// Batch lookup of display names keyed by user id.
pub async fn user_display_labels(db: &DatabaseConnection, ids: &[i64]) -> HashMap<i64, String> {
    let ids: Vec<i64> = ids.iter().copied().filter(|id| *id > 0).collect();
    if ids.is_empty() {
        return HashMap::new();
    }
    let users = UserEntity::find()
        .filter(user::Column::Id.is_in(ids))
        .all(db)
        .await
        .unwrap_or_default();
    users.into_iter().map(|row| (row.id, row.name)).collect()
}

pub fn generate_random_password(len: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789";
    let mut rng = rand::rng();
    (0..len)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub async fn create_hr_user(
    db: &DatabaseConnection,
    input: &PersonInput,
    role_name: &str,
) -> Result<i64, String> {
    create_hr_user_with_password(db, input, role_name, "").await
}

pub async fn create_hr_user_with_password<C: ConnectionTrait>(
    db: &C,
    input: &PersonInput,
    role_name: &str,
    plain_password: &str,
) -> Result<i64, String> {
    let salt = password::generate_salt();
    let hash =
        password::hash_password(plain_password.as_bytes(), &salt).map_err(|e| e.to_string())?;
    let now = Utc::now();
    let model = user::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(input.name.clone()),
        email: Set(input.email.clone().into()),
        phone: Set(input.mobile.clone().into()),
        role: Set(role_name.to_string()),
        password_hash: Set(Some(hash)),
        password_salt: Set(Some(salt)),
        last_login_time: Set(None),
        timezone: Set("Asia/Kolkata".into()),
    };
    Ok(model.insert(db).await.map_err(|e| e.to_string())?.id)
}

pub const HR_ROLES: &[&str] = roles::ALL;

/// Name, mobile, and email copied from an existing user account.
pub async fn person_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
) -> Result<PersonInput, String> {
    let user = user::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Select a user".to_string())?;
    let person = PersonInput {
        name: user.name,
        mobile: user.phone.to_string(),
        email: user.email.to_string(),
    };
    validate_person_input(&person)
        .map_err(|_| "The selected user needs a name, mobile, and email".to_string())?;
    Ok(normalized_person_input(&person))
}

pub async fn user_label<C: ConnectionTrait>(db: &C, id: Option<i64>) -> String {
    let Some(id) = id.filter(|id| *id > 0) else {
        return String::new();
    };
    match user::Entity::find_by_id(id).one(db).await {
        Ok(Some(user)) => user.name,
        _ => format!("User {id}"),
    }
}

pub async fn user_role_title<C: ConnectionTrait>(db: &C, user_id: i64) -> String {
    let Ok(Some(user)) = user::Entity::find_by_id(user_id).one(db).await else {
        return String::new();
    };
    let name = user.role.trim();
    if name.is_empty() {
        return String::new();
    }
    lariv_core::role_registry::current_role_registry().title_of(name)
}

pub async fn user_is_superuser<C: ConnectionTrait>(db: &C, user_id: i64) -> Result<bool, String> {
    let user = user::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(user.is_some_and(|user| Superuser::matches(&user.role)))
}
