//! Survey-style forms with JSON question definitions and responses.
//!
//! Data layer only: SeaORM entities and migrations. No admin UI or fill-out
//! routes in this plugin yet.
//!
//! # Database models
//!
//! - [`entities::Form`]: titled form with a JSON list of [`types::FormQuestion`].
//! - [`entities::FormResponse`]: submitted answers keyed by [`types::FormQuestionId`].

pub mod entities;
pub mod migrations;
pub mod types;

use crate::plugin_install::define_plugin_install;

/// Capability tag for the forms plugin.
pub struct FormsTag;

define_plugin_install! {
    plugin: FormsTag;
    /// Register forms database migrations.
    steps: [
        migrations(migrations::Hook),
    ]
}
