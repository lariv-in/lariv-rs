use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::plugins::users::{
    entities::{
        role::Entity as RoleEntity,
        user::{self, Entity as UserEntity, User},
    },
    error::UsersError,
    jwt, password,
    session::SESSION_TTL,
};

pub async fn authenticate(
    db: &DatabaseConnection,
    email: &str,
    plain_password: &str,
) -> Result<User, UsersError> {
    let user = UserEntity::find()
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await?
        .ok_or(UsersError::AuthFailed)?;

    let ok = password::verify_password(
        plain_password.as_bytes(),
        &user.password_salt,
        &user.password_hash,
    )?;
    if !ok {
        return Err(UsersError::AuthFailed);
    }
    Ok(user)
}

pub fn login_token(
    user: &User,
    signing_key: &[u8],
    jwt_issuer: &[u8],
) -> Result<String, UsersError> {
    jwt::issue_token(user, signing_key, jwt_issuer, SESSION_TTL)
}

pub async fn set_password(
    db: &DatabaseConnection,
    mut user: user::ActiveModel,
    plain: &str,
) -> Result<User, UsersError> {
    let salt = password::generate_salt();
    let hash = password::hash_password(plain.as_bytes(), &salt)?;
    let now = Utc::now();
    user.password_salt = Set(salt);
    user.password_hash = Set(hash);
    user.updated_at = Set(Some(now));
    Ok(user.update(db).await?)
}

pub struct CreateUser {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub plain_password: String,
    pub role_id: i64,
    pub is_superuser: bool,
    pub timezone: Option<String>,
}

pub async fn create_user(db: &DatabaseConnection, input: CreateUser) -> Result<User, UsersError> {
    let salt = password::generate_salt();
    let hash = password::hash_password(input.plain_password.as_bytes(), &salt)?;
    let now = Utc::now();
    let model = user::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(input.name),
        email: Set(input.email),
        phone: Set(input.phone),
        is_superuser: Set(input.is_superuser),
        role_id: Set(input.role_id),
        password_hash: Set(hash),
        password_salt: Set(salt),
        timezone: Set(input.timezone.unwrap_or_else(|| "Asia/Kolkata".into())),
    };
    Ok(model.insert(db).await?)
}

pub async fn role_name_for_user(
    db: &DatabaseConnection,
    user: &User,
) -> Result<String, UsersError> {
    if user.is_superuser {
        return Ok("superuser".into());
    }
    let role = RoleEntity::find_by_id(user.role_id)
        .one(db)
        .await?
        .ok_or(UsersError::NotFound)?;
    Ok(role.name)
}
