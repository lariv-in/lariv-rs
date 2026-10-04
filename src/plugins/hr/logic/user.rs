use chrono::Utc;
use rand::RngExt;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection};

use crate::plugins::hr::roles;
use crate::plugins::users::{entities::user, password};

use super::person::PersonInput;

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
        timezone: Set("Asia/Kolkata".into()),
    };
    Ok(model.insert(db).await.map_err(|e| e.to_string())?.id)
}

pub const HR_ROLES: &[&str] = roles::ALL;
