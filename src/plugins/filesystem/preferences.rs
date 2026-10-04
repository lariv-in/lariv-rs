//! Singleton access settings for the filesystem root.

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use super::entities::filesystem_preferences::{
    self, Entity as PreferencesEntity, Model as FilesystemPreferences,
};
use super::permissions::NodePermissions;

/// Load the singleton row, creating open defaults when it is missing.
///
/// The default matches access before this row existed: a null owner, a null role,
/// and [`NodePermissions::legacy`] so every principal can view, change, and open the root.
pub async fn load(db: &DatabaseConnection) -> Result<FilesystemPreferences, sea_orm::DbErr> {
    if let Some(row) = PreferencesEntity::find_by_id(1).one(db).await? {
        return Ok(row);
    }
    let now = Utc::now();
    let model = filesystem_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        owner_id: Set(None),
        role: Set(None),
        permissions: Set(NodePermissions::legacy()),
    };
    Ok(model.insert(db).await?)
}

/// Replace owner, role, and access on the filesystem root.
pub async fn save(
    db: &DatabaseConnection,
    owner_id: Option<i64>,
    role: Option<String>,
    permissions: NodePermissions,
) -> Result<FilesystemPreferences, sea_orm::DbErr> {
    let mut row: filesystem_preferences::ActiveModel = load(db).await?.into();
    row.owner_id = Set(owner_id);
    row.role = Set(role);
    row.permissions = Set(permissions);
    row.updated_at = Set(Some(Utc::now()));
    Ok(row.update(db).await?)
}
