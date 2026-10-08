//! Request-scoped HR sidebar extras.

use sea_orm::DatabaseConnection;

use crate::logic::leave::user_is_leave_manager;

tokio::task_local! {
    static LEAVE_MANAGER: bool;
}

/// True when this render should offer the manager leave-approval page.
pub fn leave_approvals_visible() -> bool {
    LEAVE_MANAGER.try_with(|value| *value).unwrap_or(false)
}

/// Run `render` with the manager-approval link shown when `user_id` manages someone.
pub async fn hr_page<T>(db: &DatabaseConnection, user_id: i64, render: impl FnOnce() -> T) -> T {
    let manager = user_is_leave_manager(db, user_id).await;
    LEAVE_MANAGER.sync_scope(manager, render)
}
