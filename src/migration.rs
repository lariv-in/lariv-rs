//! Database migration capability — plugins register SeaORM migrators; CLI runs them as one composite `up`.
//!
//! SeaORM tracks applied migrations globally in `seaql_migrations`. Calling each plugin's
//! [`MigratorTrait::up`] separately fails because every row must appear in *that* migrator's
//! file list. This module collects all plugin migrations and runs them through a temporary
//! composite migrator in install order.
//!
//! # Lifecycle
//!
//! 1. Attach an empty migration capability via [`with_migrations`].
//! 2. Plugins queue [`MigrationRegistrar`] hooks during install (or use `define_register_migrations!`).
//! 3. At mount, hooks fold over the migrator HList → [`MigrationCapability`].
//! 4. [`run_migrations`] (or the `migrate` CLI command) runs all migrators against [`DbTag`].
//!
//! # Core types
//!
//! - [`MigrationTag`] — capability tag
//! - [`MigrationCapability`] — mounted HList of tagged [`MigratorTrait`] values
//! - [`MigrationCap`] — builder-phase [`CapStore`]
//! - [`MigrationRegistrar`] — plugin hook trait
//! - [`RunMigrations`] — async fold that executes collected migrations
//!
//! # Examples
//!
//! ```rust ignore
//! define_register_migrations! {
//!     plugin: BlogTag;
//!     migrator: Migrator;
//! }
//!
//! let app = with_migrations(app);
//! // After mount:
//! run_migrations(&mounted_app).await?;
//! ```

use frunk::{HCons, HNil, hlist::HList};
use sea_orm::{DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;

use crate::{
    app::{App, MountedApp},
    capability::{
        ApplyHooks, CapStore, Capability, FoldRegistrarHooks, apply_registrar_hook,
        mount_with_hooks,
    },
    db::{DbState, DbTag},
    tag::Tagged,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::GetByTag,
    },
};

/// Capability tag for the migration hook.
pub struct MigrationTag;

/// Mounted migration capability: a compile-time HList of tagged [`MigratorTrait`] values.
#[derive(Clone)]
pub struct MigrationCapability<Migrators> {
    pub migrators: Migrators,
}

impl MigrationCapability<HNil> {
    /// Empty migrator list (starting point for [`MigrationRegistrar`] hooks).
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
    /// Prepend a tagged migrator (head of the HList = most recently registered).
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

    /// Run all registered migrators against the given database connection.
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
    /// Eagerly fold registrar hooks into items (testing / pre-mount inspection).
    pub fn resolve_hooks(
        self,
    ) -> MigrationCap<HNil, <Hooks as FoldRegistrarHooks<MigrationTag, Items>>::Output>
    where
        Hooks: FoldRegistrarHooks<MigrationTag, Items>,
    {
        CapStore::with_items(self.hooks.fold_registrar_hooks(self.items))
    }
}

/// Plugin hook for appending migrators onto a [`MigrationCapability`].
pub trait MigrationRegistrar<M>: Sized {
    type Output;
    fn register_migrations(self, cap: MigrationCapability<M>) -> MigrationCapability<Self::Output>;
}

apply_registrar_hook! {
    capability: MigrationCapability;
    trait: MigrationRegistrar;
    method: register_migrations;
    field: migrators;
    proof: crate::capability::MigrationHookProof;
    tag: MigrationTag;
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

/// Collect [`MigrationTrait`](sea_orm_migration::MigrationTrait) boxes from each plugin migrator (tail first = install order).
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

/// Register a plugin migrator via a local hook type.
///
/// ```ignore
/// define_register_migrations! {
///     plugin: BlogTag;
///     migrator: Migrator;
/// }
/// ```
#[macro_export]
macro_rules! define_register_migrations {
    (
        plugin: $plugin:ty;
        migrator: $migrator:ty;
    ) => {
        #[derive(Clone, Copy, Default)]
        pub struct Hook;

        impl<M> $crate::migration::MigrationRegistrar<M> for Hook
        where
            M: ::frunk::hlist::HList + Clone + $crate::migration::CollectMigrations + Send,
        {
            type Output =
                impl ::frunk::hlist::HList + $crate::migration::CollectMigrations + Clone + Send;

            fn register_migrations(
                self,
                cap: $crate::migration::MigrationCapability<M>,
            ) -> $crate::migration::MigrationCapability<Self::Output> {
                cap.prepend::<$plugin, _>(<$migrator>::default())
            }
        }
    };
}

/// Attach an empty migration capability to the app builder.
///
/// Plugins register migrators via [`MigrationRegistrar`] hooks during install.
pub fn with_migrations<L, Proof>(app: App<L>) -> App<HCons<MigrationCap<HNil, HNil>, L>>
where
    L: HList + CapTagAbsent<MigrationTag, Proof>,
{
    app.add_capability(CapStore::with_items(HNil))
}

/// Run all migrators registered on [`MigrationTag`], using the connection from [`DbTag`].
///
/// Requires both capabilities to be present on the mounted app. Prefer
/// [`MountedApp::run_migrations`](crate::app::MountedApp::run_migrations) in application code.
pub async fn run_migrations<M, MigIdx, DbIdx, Migrators>(app: &MountedApp<M>) -> Result<(), DbErr>
where
    M: GetByTag<MigrationTag, MigIdx, Value = MigrationCapability<Migrators>>,
    M: GetByTag<DbTag, DbIdx, Value = DbState>,
    Migrators: RunMigrations + Clone,
{
    let db = app.get_capability_output::<DbTag, DbIdx>().conn.clone();
    let migrations = app.get_capability_output::<MigrationTag, MigIdx>().clone();
    migrations.run(&db).await
}
