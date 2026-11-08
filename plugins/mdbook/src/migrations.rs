use sea_orm_migration::prelude::*;

use super::MdBookTag;

mod m20261108_000001_create_mdbook_preferences;
mod m20261108_000002_rename_mdbook_preferences;
mod m20261108_000003_mdbook_preferences_vnode_fkeys;
mod preferences_table;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261108_000001_create_mdbook_preferences::Migration),
            Box::new(m20261108_000002_rename_mdbook_preferences::Migration),
            Box::new(m20261108_000003_mdbook_preferences_vnode_fkeys::Migration),
        ]
    }
}

lariv_core::define_register_migrations! {
    plugin: MdBookTag;
    migrator: Migrator;
}
