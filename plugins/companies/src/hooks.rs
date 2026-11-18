//! Optional callbacks for deployment plugins (catalog clone after company create).

use std::sync::{Arc, Mutex};

use futures_util::future::BoxFuture;
use sea_orm::DatabaseConnection;

type AfterCreate = Arc<
    dyn Fn(DatabaseConnection, i64, i64) -> BoxFuture<'static, Result<(), String>> + Send + Sync,
>;

static AFTER_CREATE: Mutex<Option<AfterCreate>> = Mutex::new(None);

/// Register work to run after a company and its owner membership exist.
pub fn set_after_create_company<F>(hook: Option<F>)
where
    F: Fn(DatabaseConnection, i64, i64) -> BoxFuture<'static, Result<(), String>>
        + Send
        + Sync
        + 'static,
{
    *AFTER_CREATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) =
        hook.map(|hook| Arc::new(hook) as AfterCreate);
}

pub(crate) async fn run_after_create(
    db: DatabaseConnection,
    company_id: i64,
    owner_user_id: i64,
) -> Result<(), String> {
    let hook = AFTER_CREATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    match hook {
        Some(hook) => hook(db, company_id, owner_user_id).await,
        None => Ok(()),
    }
}
