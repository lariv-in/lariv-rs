use sea_orm::EntityTrait;

use crate::plugins::tasks::entities::task_status::Entity as TaskStatusEntity;
use crate::plugins::tasks::scope::find_status_scoped;
use crate::plugins::users::state::AuthContext;

pub async fn delete_status(
    db: &sea_orm::DatabaseConnection,
    status_id: i64,
    auth: &AuthContext,
) -> Result<(), String> {
    let existing = find_status_scoped(db, status_id, auth)
        .await
        .ok_or_else(|| "status not found".to_string())?;
    TaskStatusEntity::delete_by_id(existing.id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
