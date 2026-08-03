//! Shared SeaORM database connection capability.
//!
//! The [`DbTag`] capability holds a single [`DatabaseConnection`] shared across all
//! plugins. Connect during [`App::load_config`](crate::app::App::load_config) or
//! attach manually via [`with_db`].
//!
//! Pool sizing with 100 max connections, 25 min idle,
//! 1h max lifetime, 15m max idle time.

use std::time::Duration;

use frunk::{HCons, HNil, hlist::HList};
use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};

use crate::{
    app::App,
    capability::{CapStore, Capability},
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};

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
