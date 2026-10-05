//! SeaORM entity models (`entities/`).
//!
//! # Defining entities
//!
//! Database tables are SeaORM entities in `entities/` (typically one file per table):
//!
//! ```ignore
//! // entities/item.rs
//! use sea_orm::entity::prelude::*;
//!
//! #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
//! #[sea_orm(table_name = "items")]
//! pub struct Model {
//!     #[sea_orm(primary_key)]
//!     pub id: i64,
//!     pub name: String,
//!     pub created_at: chrono::DateTime<chrono::Utc>,
//! }
//!
//! #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
//! pub enum Relation {}
//!
//! impl ActiveModelBehavior for ActiveModel {}
//! ```
//!
//! Re-export from `entities/mod.rs`:
//!
//! ```ignore
//! pub mod item;
//! pub use item::Entity as ItemEntity;
//! pub use item::Model as Item;
//! ```
//!
//! # Using entities in handlers and layers
//!
//! ```ignore
//! use sea_orm::{EntityTrait, QueryOrder};
//! use super::entities::ItemEntity;
//!
//! let rows = ItemEntity::find()
//!     .order_by_desc(item::Column::CreatedAt)
//!     .all(&state.db)
//!     .await?;
//! ```
//!
//! # Relations
//!
//! Define `Relation` enum variants for foreign keys and use `.find_also_related()`
//! or `.find_with_related()` in query patchers or loaders.
//!
//! # Migrations
//!
//! Entities describe the Rust side; schema changes go through SeaORM migrations — see
//! [`super::migrations`]. Run `cargo run -- migrate` after adding migrations.
