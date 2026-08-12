//! Migration helpers for idempotent schema creation.

use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

pub fn is_postgres(manager: &SchemaManager<'_>) -> bool {
    manager.get_connection().get_database_backend() == DbBackend::Postgres
}
