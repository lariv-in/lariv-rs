use sea_orm::EntityTrait;

use crate::plugins::tasks::entities::task::Entity as TaskEntity;
use crate::plugins::tasks::scope::find_task_scoped;
use crate::plugins::users::state::AuthContext;

pub async fn delete_task(
    db: &sea_orm::DatabaseConnection,
    task_id: i64,
    auth: &AuthContext,
) -> Result<(), String> {
    let existing = find_task_scoped(db, task_id, auth)
        .await
        .ok_or_else(|| "task not found".to_string())?;
    TaskEntity::delete_by_id(existing.id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
