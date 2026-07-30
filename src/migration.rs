use frunk::{HCons, HNil, hlist::HList};
use sea_orm::{DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;

use crate::{
    app::{App, MountedApp},
    capability::{ApplyHooks, CapStore, Capability, apply_register_hook, mount_with_hooks},
    db::{DbState, DbTag},
    hooks::zst_hook,
    tag::Tagged,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::GetByTag,
    },
};

/// Capability tag for the migration hook.
pub struct MigrationTag;

zst_hook!(
    /// Deferred plugin hook: register migrators at mount time.
    RegisterMigrationsHook
);

/// Mounted migration capability: a compile-time HList of tagged [`MigratorTrait`] values.
#[derive(Clone)]
pub struct MigrationCapability<Migrators> {
    pub migrators: Migrators,
}

impl MigrationCapability<HNil> {
    pub fn new() -> Self {
        Self { migrators: HNil }
    }
}

impl Default for MigrationCapability<HNil> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Migrators> MigrationCapability<Migrators> {
    pub fn prepend<Tag, M>(
        self,
        migrator: M,
    ) -> MigrationCapability<HCons<Tagged<Tag, M>, Migrators>>
    where
        Migrators: HList,
        M: MigratorTrait + Clone,
    {
        MigrationCapability {
            migrators: HCons {
                head: Tagged::new(migrator),
                tail: self.migrators,
            },
        }
    }

    pub async fn run(self, db: &DatabaseConnection) -> Result<(), DbErr>
    where
        Migrators: RunMigrations,
    {
        self.migrators.run_migrations(db).await
    }
}

/// Builder-phase migration capability.
pub type MigrationCap<Hooks, Items> = CapStore<MigrationTag, Hooks, Items>;

impl<Hooks, Items> MigrationCap<Hooks, Items> {
    pub fn resolve_hooks<Proof>(
        self,
    ) -> MigrationCap<HNil, <Hooks as ApplyHooks<Items, Proof>>::Output>
    where
        Hooks: ApplyHooks<Items, Proof>,
    {
        CapStore::with_items(self.hooks.apply_hooks(self.items))
    }
}

apply_register_hook! {
    hook: RegisterMigrationsHook;
    capability: MigrationCapability;
    trait: RegisterMigrations;
    method: register_migrations;
    field: migrators;
}

impl<Hooks, Items> Capability for MigrationCap<Hooks, Items>
where
    Hooks: ApplyHooks<Items>,
{
    type Value = MigrationCapability<Hooks::Output>;
    type Output = Tagged<MigrationTag, MigrationCapability<Hooks::Output>>;
    type Hooks = Hooks;
    type Items = Items;

    fn mount(self) -> Self::Output {
        mount_with_hooks(self, |items| MigrationCapability { migrators: items })
    }
}

/// Plugin hook for appending migrators onto a [`MigrationCapability`].
pub trait RegisterMigrations<Plugin, Proof = ()>: Sized {
    type Output;
    fn register_migrations(self) -> Self::Output;
}

/// Fold a migrators HList into one SeaORM `up` so every applied version is known.
///
/// Calling each plugin [`MigratorTrait::up`] separately fails: SeaORM requires every
/// row in `seaql_migrations` to appear in *that* migrator's file list.
pub trait RunMigrations {
    fn run_migrations(
        self,
        db: &DatabaseConnection,
    ) -> impl std::future::Future<Output = Result<(), DbErr>> + Send;
}

/// Collect [`MigrationTrait`] boxes from each plugin migrator (tail first = install order).
pub trait CollectMigrations {
    fn collect_migrations(self) -> Vec<Box<dyn sea_orm_migration::MigrationTrait>>;
}

impl CollectMigrations for HNil {
    fn collect_migrations(self) -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
        Vec::new()
    }
}

impl<Tag, M, Tail> CollectMigrations for HCons<Tagged<Tag, M>, Tail>
where
    M: MigratorTrait,
    Tail: CollectMigrations,
{
    fn collect_migrations(self) -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
        let mut migrations = self.tail.collect_migrations();
        migrations.extend(M::migrations());
        migrations
    }
}

thread_local! {
    static COMPOSITE_MIGRATIONS: std::cell::RefCell<
        Option<Vec<Box<dyn sea_orm_migration::MigrationTrait>>>,
    > = const { std::cell::RefCell::new(None) };
}

/// Temporary migrator that yields the collected plugin migrations for one `up` call.
struct CompositeMigrator;

impl MigratorTrait for CompositeMigrator {
    fn migrations() -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
        COMPOSITE_MIGRATIONS.with(|cell| cell.borrow_mut().take().unwrap_or_default())
    }
}

impl<L> RunMigrations for L
where
    L: CollectMigrations + Send,
{
    async fn run_migrations(self, db: &DatabaseConnection) -> Result<(), DbErr> {
        let migrations = self.collect_migrations();
        COMPOSITE_MIGRATIONS.with(|cell| {
            *cell.borrow_mut() = Some(migrations);
        });
        CompositeMigrator::up(db, None).await
    }
}

pub fn with_migrations<L, Proof>(app: App<L>) -> App<HCons<MigrationCap<HNil, HNil>, L>>
where
    L: HList + CapTagAbsent<MigrationTag, Proof>,
{
    app.add_capability(CapStore::with_items(HNil))
}

/// Run all migrators registered on [`MigrationTag`], using [`DbTag`].
pub async fn run_migrations<M, MigIdx, DbIdx, Migrators>(
    app: &MountedApp<M>,
) -> Result<(), DbErr>
where
    M: GetByTag<MigrationTag, MigIdx, Value = MigrationCapability<Migrators>>,
    M: GetByTag<DbTag, DbIdx, Value = DbState>,
    Migrators: RunMigrations + Clone,
{
    let db = app.get_capability_output::<DbTag, DbIdx>().conn.clone();
    let migrations = app.get_capability_output::<MigrationTag, MigIdx>().clone();
    migrations.run(&db).await
}
