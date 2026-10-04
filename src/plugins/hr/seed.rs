//! HR startup seed. Roles are compile-time types, so there is nothing to insert.

use sea_orm::DatabaseConnection;

pub async fn seed(_db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    Ok(())
}
