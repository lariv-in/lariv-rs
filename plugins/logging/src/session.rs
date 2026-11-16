//! Stamp `app.user_id` on every pooled Postgres connection so audit triggers can
//! record who wrote the row.

use std::sync::Once;

use futures_util::FutureExt;
use sea_orm::sqlx;

static HOOK: Once = Once::new();

/// Register the checkout hook. Call this before [`lariv_core::app::App::load_config`],
/// which is what opens the pool.
pub fn install_session_hook() {
    HOOK.call_once(|| {
        lariv_core::db::set_db_actor_probe(|| {
            lariv_plugin_users::role_authorization::current_auth().map(|auth| auth.user.id)
        });
        lariv_core::db::set_postgres_before_acquire(Some(session_hook));
    });
}

fn session_hook<'c>(
    conn: &'c mut sqlx::postgres::PgConnection,
    _meta: sqlx::pool::PoolConnectionMetadata,
) -> futures_util::future::BoxFuture<'c, Result<bool, sqlx::Error>> {
    async move {
        let actor = lariv_core::db::db_actor().or_else(|| {
            lariv_plugin_users::role_authorization::current_auth().map(|auth| auth.user.id)
        });
        let value = actor.map(|id| id.to_string()).unwrap_or_default();
        sqlx::query("SELECT set_config('app.user_id', $1, false)")
            .bind(value)
            .execute(&mut *conn)
            .await?;
        Ok(true)
    }
    .boxed()
}
