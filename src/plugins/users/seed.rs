//! Startup seed helpers for the users plugin (roles / default admin).

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::plugins::users::{
    auth, config::UsersConfig,
    entities::{
        role::{self, Entity as RoleEntity},
        user::{self, Entity as UserEntity},
    },
    error::UsersError,
    state::UsersState,
};

pub const UNASSIGNED_ROLE: &str = "unassigned";
pub const UNASSIGNED_ROLE_ID: i64 = 1;

pub async fn ensure_unassigned_role(db: &DatabaseConnection) -> Result<role::Model, UsersError> {
    if let Some(existing) = RoleEntity::find_by_id(UNASSIGNED_ROLE_ID).one(db).await? {
        return Ok(existing);
    }
    if let Some(by_name) = RoleEntity::find()
        .filter(role::Column::Name.eq(UNASSIGNED_ROLE))
        .one(db)
        .await?
    {
        return Ok(by_name);
    }

    let now = Utc::now();
    // Prefer inserting with id=1 when the table is empty.
    let model = role::ActiveModel {
        id: Set(UNASSIGNED_ROLE_ID),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(UNASSIGNED_ROLE.into()),
    };
    match model.insert(db).await {
        Ok(role) => Ok(role),
        Err(_) => {
            // Fallback without forced id (e.g. sqlite autoincrement quirks)
            let model = role::ActiveModel {
                id: Default::default(),
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                name: Set(UNASSIGNED_ROLE.into()),
            };
            Ok(model.insert(db).await?)
        }
    }
}

pub async fn ensure_admin(
    db: &DatabaseConnection,
    config: &UsersConfig,
) -> Result<Option<user::Model>, UsersError> {
    if config.admin_email.is_empty() || config.admin_password.is_empty() {
        return Ok(None);
    }

    if let Some(existing) = UserEntity::find()
        .filter(user::Column::Email.eq(&config.admin_email))
        .one(db)
        .await?
    {
        return Ok(Some(existing));
    }

    let role = ensure_unassigned_role(db).await?;
    let user = auth::create_user(
        db,
        auth::CreateUser {
            name: "Admin".into(),
            email: config.admin_email.clone(),
            phone: format!("admin-{}", config.admin_email),
            plain_password: config.admin_password.clone(),
            role_id: role.id,
            is_superuser: true,
            timezone: None,
        },
    )
    .await?;
    Ok(Some(user))
}

// Ensure unassigned role and optional configured admin user.
pub async fn seed(state: &UsersState) -> Result<(), UsersError> {
    ensure_unassigned_role(&state.db).await?;
    ensure_admin(&state.db, &state.config).await?;
    Ok(())
}
