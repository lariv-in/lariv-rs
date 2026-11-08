//! Singleton mdBook settings stored in `mdbook_preferences`.

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, DbErr, EntityTrait};

use lariv_plugin_filesystem::{node, storage::DynFilestore};

use super::entities::MdBookPreferences;
use super::entities::mdbook_preferences::{self, Entity as PreferencesEntity};

pub const DEFAULT_SOURCE_DIR: &str = "mdbook";
pub const DEFAULT_OUTPUT_DIR: &str = "mdbook-site";
pub const DEFAULT_URL_PREFIX: &str = "/book";

/// Preferences used before a row has been saved, and by tests.
///
/// `source_vnode_id` and `output_vnode_id` are `0` until a row is loaded.
pub fn default_preferences() -> MdBookPreferences {
    MdBookPreferences {
        id: 1,
        created_at: None,
        updated_at: None,
        source_vnode_id: 0,
        output_vnode_id: 0,
        url_prefix: DEFAULT_URL_PREFIX.into(),
        build_on_serve: true,
    }
}

/// Load the singleton row, inserting the defaults when it is missing.
///
/// The first insert creates root directories named [`DEFAULT_SOURCE_DIR`] and
/// [`DEFAULT_OUTPUT_DIR`] and stores their vnode ids.
pub async fn load_preferences(
    db: &DatabaseConnection,
    store: &DynFilestore,
) -> Result<MdBookPreferences, DbErr> {
    if let Some(row) = PreferencesEntity::find_by_id(1).one(db).await? {
        return Ok(row);
    }
    let now = Utc::now();
    let defaults = default_preferences();
    let source_vnode_id = ensure_root_directory(db, store, DEFAULT_SOURCE_DIR).await?;
    let output_vnode_id = ensure_root_directory(db, store, DEFAULT_OUTPUT_DIR).await?;
    let model = mdbook_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        source_vnode_id: Set(source_vnode_id),
        output_vnode_id: Set(output_vnode_id),
        url_prefix: Set(defaults.url_prefix),
        build_on_serve: Set(defaults.build_on_serve),
    };
    model.insert(db).await
}

/// Replace the singleton row with `prefs`.
pub async fn save_preferences(
    db: &DatabaseConnection,
    store: &DynFilestore,
    prefs: &MdBookPreferences,
) -> Result<MdBookPreferences, DbErr> {
    let mut row: mdbook_preferences::ActiveModel = load_preferences(db, store).await?.into();
    row.source_vnode_id = Set(prefs.source_vnode_id);
    row.output_vnode_id = Set(prefs.output_vnode_id);
    row.url_prefix = Set(prefs.url_prefix.clone());
    row.build_on_serve = Set(prefs.build_on_serve);
    row.updated_at = Set(Some(Utc::now()));
    row.update(db).await
}

async fn ensure_root_directory(
    db: &DatabaseConnection,
    store: &DynFilestore,
    name: &str,
) -> Result<i64, DbErr> {
    node::ensure_directory_path(db, store, None, &[name.to_string()])
        .await
        .map_err(|error| DbErr::Custom(error.to_string()))?
        .ok_or_else(|| DbErr::Custom(format!("could not create directory \"{name}\"")))
}

pub fn id_text(id: i64) -> String {
    if id > 0 {
        id.to_string()
    } else {
        String::new()
    }
}

pub fn parse_vnode_id(raw: &str, label: &str) -> Result<i64, String> {
    let id = raw.trim().parse::<i64>().unwrap_or(0);
    if id > 0 {
        Ok(id)
    } else {
        Err(format!("{label} is required"))
    }
}
