//! Shared SeaORM database connection capability.
//!
//! The [`DbTag`] capability holds a single [`DatabaseConnection`] shared across all
//! plugins. Connect during [`App::load_config`](crate::app::App::load_config) or
//! attach manually via [`with_db`].
//!
//! Pool sizing with 100 max connections, 25 min idle,
//! 1h max lifetime, 15m max idle time.

use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use frunk::{HCons, HNil, hlist::HList};
use futures_util::future::BoxFuture;
use sea_orm::sqlx;
use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};

tokio::task_local! {
    static DB_ACTOR: i64;
}

type PgBeforeAcquire = Arc<
    dyn for<'c> Fn(
            &'c mut sqlx::postgres::PgConnection,
            sqlx::pool::PoolConnectionMetadata,
        ) -> BoxFuture<'c, Result<bool, sqlx::Error>>
        + Send
        + Sync,
>;

static PG_BEFORE_ACQUIRE: Mutex<Option<PgBeforeAcquire>> = Mutex::new(None);

type DbActorProbe = fn() -> Option<i64>;

static DB_ACTOR_PROBE: Mutex<Option<DbActorProbe>> = Mutex::new(None);

fn hook_lock() -> std::sync::MutexGuard<'static, Option<PgBeforeAcquire>> {
    PG_BEFORE_ACQUIRE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// User id stamped onto Postgres connections checked out inside this scope.
///
/// [`connect`] reads it from the optional `before_acquire` hook. Outside the scope
/// the hook clears the setting, so a pooled connection cannot reuse another request's id.
pub fn db_actor() -> Option<i64> {
    DB_ACTOR.try_with(|id| *id).ok()
}

/// Remember how this process finds the signed-in user on the request thread.
///
/// Rune scripts run on a blocking thread, where the request's task-local auth is
/// not visible. [`db_actor_resolved`] reads this probe before that hop.
pub fn set_db_actor_probe(probe: fn() -> Option<i64>) {
    *DB_ACTOR_PROBE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(probe);
}

/// Signed-in user on this thread: the [`db_actor`] scope, or the registered probe.
pub fn db_actor_resolved() -> Option<i64> {
    if let Some(id) = db_actor() {
        return Some(id);
    }
    DB_ACTOR_PROBE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .and_then(|probe| probe())
}

/// Run `fut` with [`db_actor`] set to `user_id`.
pub async fn with_db_actor<T>(user_id: i64, fut: impl Future<Output = T>) -> T {
    DB_ACTOR.scope(user_id, fut).await
}

/// Install the Postgres checkout hook used by later [`connect`] calls.
///
/// The hook runs on the serving pool. Pass `None` to clear it. Register it before
/// [`crate::app::App::load_config`], which is what opens the pool.
pub fn set_postgres_before_acquire<F>(hook: Option<F>)
where
    F: for<'c> Fn(
            &'c mut sqlx::postgres::PgConnection,
            sqlx::pool::PoolConnectionMetadata,
        ) -> BoxFuture<'c, Result<bool, sqlx::Error>>
        + Send
        + Sync
        + 'static,
{
    *hook_lock() = hook.map(|hook| Arc::new(hook) as PgBeforeAcquire);
}

use crate::{
    app::App,
    capability::{CapStore, Capability},
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};

pub mod location;
pub mod migration_sql;
pub mod null_text;
pub mod phone;
#[cfg(feature = "plugin-users")]
pub mod phonenumber;
pub mod trigram;

/// Capability tag for the shared SeaORM connection.
pub struct DbTag;

/// Mounted database state wrapping the SeaORM connection pool.
#[derive(Clone)]
pub struct DbState {
    /// The shared SeaORM connection used by all plugins.
    pub conn: DatabaseConnection,
}

/// Open a SeaORM connection to `database_url`.
///
/// # Examples
///
/// ```ignore
/// let conn = lariv_rs::db::connect("sqlite://data/lariv.db?mode=rwc").await?;
/// ```
pub async fn connect(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    let mut opt = ConnectOptions::new(database_url.to_owned());
    opt.max_connections(100)
        .min_connections(1)
        .max_lifetime(Duration::from_secs(3600))
        .idle_timeout(Duration::from_secs(900))
        .sqlx_logging(false);
    if let Some(hook) = hook_lock().clone() {
        opt.map_sqlx_postgres_before_acquire(move |conn, meta| hook(conn, meta));
    }
    Database::connect(opt).await
}

/// Builder-phase DB capability.
pub type DbCap = CapStore<DbTag, HNil, DbState>;

impl Capability for DbCap {
    type Value = DbState;
    type Output = Tagged<DbTag, DbState>;
    type Hooks = HNil;
    type Items = DbState;

    fn mount(self) -> Self::Output {
        Tagged::new(self.items)
    }
}

/// Attach a database connection to the app.
///
/// Called automatically by [`App::load_config`](crate::app::App::load_config).
pub fn with_db<L, Proof>(app: App<L>, conn: DatabaseConnection) -> App<HCons<DbCap, L>>
where
    L: HList + CapTagAbsent<DbTag, Proof>,
{
    app.add_capability(CapStore::with_items(DbState { conn }))
}

impl DbState {
    /// Borrow the underlying SeaORM connection.
    pub fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }
}
