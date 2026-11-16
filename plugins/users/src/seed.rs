//! Startup seed helpers for the users plugin (default admin).

use sea_orm::DatabaseConnection;

use crate::{
    auth, config::UsersConfig, entities::user, error::UsersError, roles::Superuser,
    state::UsersState,
};

pub async fn ensure_admin(
    db: &DatabaseConnection,
    config: &UsersConfig,
) -> Result<Option<user::Model>, UsersError> {
    if config.admin_email.is_empty() || config.admin_password.is_empty() {
        return Ok(None);
    }

    if let Some(existing) = auth::find_by_email(db, &config.admin_email).await? {
        return Ok(Some(existing));
    }

    let user = auth::create_user(
        db,
        auth::CreateUser {
            name: "Admin".into(),
            email: config.admin_email.clone(),
            phone: format!("admin-{}", config.admin_email),
            plain_password: config.admin_password.clone(),
            role: Superuser::NAME.into(),
            timezone: None,
        },
    )
    .await?;
    Ok(Some(user))
}

/// Seed the configured admin user after mount.
pub async fn seed(state: &UsersState) -> Result<(), UsersError> {
    ensure_admin(&state.db, &state.config).await?;
    Ok(())
}
