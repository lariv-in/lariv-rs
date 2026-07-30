use frunk::{HCons, HNil, hlist::HList};
use sea_orm::{Database, DatabaseConnection, DbErr};

use crate::{
    app::App,
    capability::{CapStore, Capability},
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};

/// Capability tag for the shared SeaORM connection.
pub struct DbTag;

#[derive(Clone)]
pub struct DbState {
    pub conn: DatabaseConnection,
}

pub async fn connect(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    Database::connect(database_url).await
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

pub fn with_db<L, Proof>(app: App<L>, conn: DatabaseConnection) -> App<HCons<DbCap, L>>
where
    L: HList + CapTagAbsent<DbTag, Proof>,
{
    app.add_capability(CapStore::with_items(DbState { conn }))
}

impl DbState {
    pub fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }
}
