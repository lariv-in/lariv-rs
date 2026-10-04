use sea_orm_migration::prelude::*;

use super::FilesystemTag;

mod m20260730_000002_create_filesystem_nodes;
mod m20260808_000001_filesystem_drop_deleted_at;
mod m20260928_000001_filesystem_node_permissions;
mod m20261003_000001_filesystem_root_permissions;
mod m20261003_000002_role_names;
mod m20261003_000003_users_drop_is_superuser;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260730_000002_create_filesystem_nodes::Migration),
            Box::new(m20260808_000001_filesystem_drop_deleted_at::Migration),
            Box::new(m20260928_000001_filesystem_node_permissions::Migration),
            Box::new(m20261003_000001_filesystem_root_permissions::Migration),
            Box::new(m20261003_000002_role_names::Migration),
            Box::new(m20261003_000003_users_drop_is_superuser::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: FilesystemTag;
    migrator: Migrator;
}
