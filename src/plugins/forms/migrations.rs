use sea_orm_migration::prelude::*;

use super::FormsTag;

mod m00001_create_forms;
mod m00002_form_uid_access_status;
mod m00003_form_appearance;
mod m00004_rename_theme_color_to_accent;

#[derive(Clone, Copy, Default)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m00001_create_forms::Migration),
            Box::new(m00002_form_uid_access_status::Migration),
            Box::new(m00003_form_appearance::Migration),
            Box::new(m00004_rename_theme_color_to_accent::Migration),
        ]
    }
}

crate::define_register_migrations! {
    plugin: FormsTag;
    migrator: Migrator;
}
