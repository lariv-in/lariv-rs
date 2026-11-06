//! Shared user id lookups for display labels and existence checks.

use sea_orm::{DatabaseConnection, EntityTrait};

use crate::entities::user::Entity as UserEntity;

/// Whether a positive user id exists.
pub async fn user_exists(db: &DatabaseConnection, id: i64) -> bool {
    if id <= 0 {
        return false;
    }
    lariv_core::web::opt_or_log(UserEntity::find_by_id(id).one(db).await, "find user by id")
        .is_some()
}

/// Display name for a users-table id (empty when missing or `id` is not positive).
pub async fn user_display_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    lariv_core::web::opt_or_log(UserEntity::find_by_id(id).one(db).await, "find user by id")
        .map(|u| u.name)
        .unwrap_or_default()
}
