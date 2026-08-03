//! Database migrations with SeaORM.
//!
//! # Migration modules
//!
//! Each plugin keeps migrators in `migrations/` as Rust modules implementing
//! [`MigrationTrait`](sea_orm_migration::MigrationTrait):
//!
//! ```ignore
//! // migrations/m20260801_000001_create_items.rs
//! use sea_orm_migration::{prelude::*, schema::*};
//!
//! #[derive(DeriveMigrationName)]
//! pub struct Migration;
//!
//! #[async_trait::async_trait]
//! impl MigrationTrait for Migration {
//!     async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
//!         manager
//!             .create_table(
//!                 Table::create()
//!                     .table(Items::Table)
//!                     .col(pk_auto(Items::Id))
//!                     .col(string(Items::Name))
//!                     .col(timestamp(Items::CreatedAt))
//!                     .to_owned(),
//!             )
//!             .await
//!     }
//!
//!     async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
//!         manager.drop_table(Table::drop().table(Items::Table).to_owned()).await
//!     }
//! }
//!
//! #[derive(DeriveIden)]
//! enum Items {
//!     Table,
//!     Id,
//!     Name,
//!     CreatedAt,
//! }
//! ```
//!
//! # Registering migrations
//!
//! ```ignore
//! // migrations/mod.rs
//! mod m20260801_000001_create_items;
//!
//! use sea_orm_migration::MigratorTrait;
//!
//! pub struct Migrator;
//!
//! impl MigratorTrait for Migrator {
//!     fn migrations() -> Vec<Box<dyn MigrationTrait>> {
//!         vec![Box::new(m20260801_000001_create_items::Migration)]
//!     }
//! }
//!
//! define_register_migrations! {
//!     plugin: MyPluginTag;
//!     migrator: Migrator;
//! }
//! ```
//!
//! Add `migrations(migrations::Hook)` to [`define_plugin_install!`](crate::plugin_install::define_plugin_install).
//!
//! # Running migrations
//!
//! Lariv merges all plugin migrators into one composite migrator. Apply pending migrations:
//!
//! ```text
//! cargo run -- migrate
//! ```
//!
//! Or call [`MountedApp::run_migrations`](crate::app::MountedApp::run_migrations) programmatically
//! before `serve`.
//!
//! # Migration tracking
//!
//! Applied revisions are recorded in the `seaql_migrations` table. All plugins share this
//! table — the framework runs every registered migrator through a single composite pass.
