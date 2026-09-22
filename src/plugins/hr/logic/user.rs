use chrono::Utc;
use rand::RngExt;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter,
};

use crate::plugins::users::{
    entities::{
        role::{self, Entity as RoleEntity},
        user::{self, Entity as UserEntity},
    },
    password,
};

use super::person::PersonInput;
use crate::plugins::hr::roles;

pub async fn role_id_for<C: ConnectionTrait>(db: &C, role_name: &str) -> Result<i64, String> {
    RoleEntity::find()
        .filter(role::Column::Name.eq(role_name))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .map(|role| role.id)
        .ok_or_else(|| format!("role not found: {role_name}"))
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
    let role_id = role_id_for(db, role_name).await?;
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
        is_superuser: Set(false),
        role_id: Set(role_id),
        password_hash: Set(Some(hash)),
        password_salt: Set(Some(salt)),
        timezone: Set("Asia/Kolkata".into()),
    };
    Ok(model.insert(db).await.map_err(|e| e.to_string())?.id)
}

pub async fn set_user_role<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    role_name: &str,
) -> Result<(), String> {
    let role_id = role_id_for(db, role_name).await?;
    let existing = UserEntity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "user not found".to_string())?;
    let mut am: user::ActiveModel = existing.into();
    am.role_id = Set(role_id);
    am.update(db).await.map_err(|e| e.to_string())?;
    Ok(())
}

pub const HR_ROLES: &[&str] = roles::ALL;
