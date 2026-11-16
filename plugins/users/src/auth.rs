use std::sync::{Arc, Mutex};

use chrono::Utc;
use sea_orm::sqlx::Executor;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{Set, Unchanged},
    ColumnTrait, DatabaseBackend, DatabaseConnection, DbErr, EntityTrait, QueryFilter,
};

use crate::{
    entities::user::{self, Entity as UserEntity, User},
    error::UsersError,
    jwt, password,
    session::SESSION_TTL,
};

static DIRECTORY: Mutex<Option<Arc<dyn AuthDirectory>>> = Mutex::new(None);

fn directory() -> Option<Arc<dyn AuthDirectory>> {
    DIRECTORY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// Lookup and insert path used when row-level security hides `users` from the app role.
///
/// Password checks stay in [`authenticate`]. The directory only fetches a row by email
/// and inserts new users.
#[async_trait::async_trait]
pub trait AuthDirectory: Send + Sync {
    /// Load one user by email, including the password hash.
    async fn find_by_email(
        &self,
        db: &DatabaseConnection,
        email: &str,
    ) -> Result<Option<User>, UsersError>;

    /// Insert a user. Public signup must not be able to choose a privileged role.
    async fn create_user(
        &self,
        db: &DatabaseConnection,
        input: CreateUser,
    ) -> Result<User, UsersError>;
}

/// Replace the user lookup used by [`find_by_email`] and [`create_user`].
pub fn set_auth_directory(directory: Arc<dyn AuthDirectory>) {
    *DIRECTORY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(directory);
}

/// Load a user by email. Uses the registered [`AuthDirectory`] when one is installed.
pub async fn find_by_email(
    db: &DatabaseConnection,
    email: &str,
) -> Result<Option<User>, UsersError> {
    if let Some(directory) = directory() {
        return directory.find_by_email(db, email).await;
    }
    Ok(UserEntity::find()
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await?)
}

pub async fn authenticate(
    db: &DatabaseConnection,
    email: &str,
    plain_password: &str,
) -> Result<User, UsersError> {
    let user = find_by_email(db, email)
        .await?
        .ok_or(UsersError::AuthFailed)?;

    let ok = password::verify_password(
        plain_password.as_bytes(),
        user.password_salt.as_deref().unwrap_or(&[]),
        user.password_hash.as_deref().unwrap_or(&[]),
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

/// Stamp `last_login_time` after a successful login.
///
/// On Postgres the user id is set inside the same transaction as the update.
/// A pool checkout hook can miss the task-local user, and the row policy then
/// rejects the write.
pub async fn record_login(db: &DatabaseConnection, user_id: i64) -> Result<(), UsersError> {
    if db.get_database_backend() == DatabaseBackend::Postgres {
        return record_login_postgres(db, user_id).await;
    }
    user::ActiveModel {
        id: Unchanged(user_id),
        last_login_time: Set(Some(Utc::now())),
        ..Default::default()
    }
    .update(db)
    .await?;
    Ok(())
}

async fn record_login_postgres(db: &DatabaseConnection, user_id: i64) -> Result<(), UsersError> {
    let mut conn = db
        .get_postgres_connection_pool()
        .acquire()
        .await
        .map_err(sqlx_err)?;
    let result = async {
        conn.execute("BEGIN").await.map_err(sqlx_err)?;
        sea_orm::sqlx::query("SELECT set_config('app.user_id', $1, true)")
            .bind(user_id.to_string())
            .execute(&mut *conn)
            .await
            .map_err(sqlx_err)?;
        let updated =
            sea_orm::sqlx::query("UPDATE users SET last_login_time = now() WHERE id = $1")
                .bind(user_id)
                .execute(&mut *conn)
                .await
                .map_err(sqlx_err)?;
        if updated.rows_affected() == 0 {
            return Err(UsersError::NotFound);
        }
        conn.execute("COMMIT").await.map_err(sqlx_err)?;
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = conn.execute("ROLLBACK").await;
    }
    result
}

fn sqlx_err(err: sea_orm::sqlx::Error) -> UsersError {
    UsersError::Db(DbErr::Custom(err.to_string()))
}

pub async fn set_password(
    db: &DatabaseConnection,
    user: user::ActiveModel,
    plain: &str,
) -> Result<User, UsersError> {
    let id = match user.id {
        Set(id) | Unchanged(id) => id,
        _ => return Err(UsersError::NotFound),
    };
    let salt = password::generate_salt();
    let hash = password::hash_password(plain.as_bytes(), &salt)?;
    let now = Utc::now();
    Ok(user::ActiveModel {
        id: Unchanged(id),
        password_salt: Set(Some(salt)),
        password_hash: Set(Some(hash)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .update(db)
    .await?)
}

#[derive(Clone)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub plain_password: String,
    pub role: String,
    pub timezone: Option<String>,
}

pub async fn create_user(db: &DatabaseConnection, input: CreateUser) -> Result<User, UsersError> {
    if let Some(directory) = directory() {
        return directory.create_user(db, input).await;
    }
    insert_user_row(db, input).await
}

/// Insert a user on `db` with no directory hook.
///
/// Privileged callers (the admin connection) use this so [`create_user`] can delegate
/// without recursing into the directory.
pub async fn insert_user_row(
    db: &DatabaseConnection,
    input: CreateUser,
) -> Result<User, UsersError> {
    let salt = password::generate_salt();
    let hash = password::hash_password(input.plain_password.as_bytes(), &salt)?;
    let now = Utc::now();
    let model = user::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(input.name),
        email: Set(input.email.into()),
        phone: Set(input.phone.into()),
        role: Set(input.role),
        password_hash: Set(Some(hash)),
        password_salt: Set(Some(salt)),
        last_login_time: Set(None),
        timezone: Set(input
            .timezone
            .unwrap_or_else(|| "Asia/Kolkata".into())
            .into()),
    };
    Ok(model.insert(db).await?)
}

pub fn role_name_for_user(user: &User) -> String {
    user.role.clone()
}

/// Reject a role name that is not in the mounted catalog.
pub fn require_role(name: &str) -> Result<(), UsersError> {
    super::role_registry::current_role_registry()
        .validate(name, true)
        .map_err(UsersError::Validation)
}
