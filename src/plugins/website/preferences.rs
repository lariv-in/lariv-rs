//! Singleton website preferences (`id = 1`).

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};
use serde_json::{Value, json};

use crate::{
    grapesjs::{GrapesJsCapability, GrapesJsTheme},
    plugins::filesystem::{node, storage::DynFilestore},
};

use super::{
    entities::{
        WebsitePreferences,
        website_preferences::{self, Entity as PrefsEntity},
    },
    render::read_vnode_text,
};

/// GrapesJS theme id for assets loaded from Custom theme preference VNodes.
pub const CUSTOM_THEME_ID: &str = "p_website.custom";

/// Load singleton preferences row (`id = 1`), creating it if missing.
pub async fn load_preferences(db: &DatabaseConnection) -> Result<WebsitePreferences, sea_orm::DbErr> {
    if let Some(prefs) = PrefsEntity::find_by_id(1).one(db).await? {
        return Ok(prefs);
    }

    let now = Utc::now();
    let model = website_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        custom_theme_css_vnode_id: Set(None),
        custom_theme_js_vnode_id: Set(None),
    };
    model.insert(db).await
}

/// Persist preferences fields onto the singleton row.
pub async fn save_preferences(
    db: &DatabaseConnection,
    prefs: WebsitePreferences,
) -> Result<WebsitePreferences, sea_orm::DbErr> {
    let mut am: website_preferences::ActiveModel = load_preferences(db).await?.into();
    am.custom_theme_css_vnode_id = Set(prefs.custom_theme_css_vnode_id.filter(|&id| id > 0));
    am.custom_theme_js_vnode_id = Set(prefs.custom_theme_js_vnode_id.filter(|&id| id > 0));
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await
}

async fn load_custom_theme_text(
    db: &DatabaseConnection,
    store: &DynFilestore,
    vnode_id: Option<i64>,
    kind: &str,
) -> String {
    let Some(vnode_id) = vnode_id.filter(|&id| id > 0) else {
        return String::new();
    };
    let Some(vnode) =
        crate::web::opt_or_log(node::get_by_id(db, vnode_id).await, &format!("get custom theme {kind} vnode"))
    else {
        return String::new();
    };
    if vnode.is_directory {
        return String::new();
    }
    read_vnode_text(store, &vnode)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, vnode_id, kind, "website: failed to read custom theme asset");
            String::new()
        })
}

/// Read CSS text for the Custom theme from the configured filesystem VNode.
pub async fn load_custom_theme_css(
    db: &DatabaseConnection,
    store: &DynFilestore,
) -> String {
    let Ok(prefs) = load_preferences(db).await else {
        return String::new();
    };
    load_custom_theme_text(db, store, prefs.custom_theme_css_vnode_id, "css").await
}

/// Read JS text for the Custom theme from the configured filesystem VNode.
pub async fn load_custom_theme_js(
    db: &DatabaseConnection,
    store: &DynFilestore,
) -> String {
    let Ok(prefs) = load_preferences(db).await else {
        return String::new();
    };
    load_custom_theme_text(db, store, prefs.custom_theme_js_vnode_id, "js").await
}

/// Look up a theme, filling Custom theme CSS/JS from preferences when needed.
pub async fn resolve_theme(
    grapes: &GrapesJsCapability,
    db: &DatabaseConnection,
    store: &DynFilestore,
    theme_id: &str,
) -> Option<GrapesJsTheme> {
    let mut theme = grapes.theme(theme_id)?.clone();
    if theme_id == CUSTOM_THEME_ID {
        theme.css = load_custom_theme_css(db, store).await;
        theme.js = load_custom_theme_js(db, store).await;
    }
    Some(theme)
}

/// Builder themes JSON with Custom theme CSS/JS loaded from configured VNodes.
pub async fn themes_json_with_custom(
    grapes: &GrapesJsCapability,
    db: &DatabaseConnection,
    store: &DynFilestore,
) -> Value {
    let custom_css = load_custom_theme_css(db, store).await;
    let custom_js = load_custom_theme_js(db, store).await;
    let mut themes = grapes.themes_json();
    let Value::Array(ref mut items) = themes else {
        return themes;
    };
    for item in items.iter_mut() {
        let Value::Object(map) = item else {
            continue;
        };
        if map.get("id").and_then(|v| v.as_str()) != Some(CUSTOM_THEME_ID) {
            continue;
        }
        if custom_css.is_empty() {
            map.remove("css");
        } else {
            map.insert("css".into(), json!(custom_css));
        }
        if custom_js.is_empty() {
            map.remove("js");
        } else {
            map.insert("js".into(), json!(custom_js));
        }
    }
    themes
}
