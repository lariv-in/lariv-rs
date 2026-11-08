//! Load, save, and publish mdBook preferences.

use axum::response::{IntoResponse, Response};
use sea_orm::DatabaseConnection;

use crate::entities::MdBookPreferences;
use crate::forms::{BuildForm, PreferencesForm, checkbox_on};
use crate::paths::normalize_prefix;
use crate::preferences::{id_text, load_preferences, parse_vnode_id, save_preferences};
use crate::publish::{PublishOutcome, directories_overlap, publish};
use crate::state::MdBookState;
use crate::templates::MdBookPreferencesPage;
use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout};
use lariv_plugin_filesystem::{entities::VNode, node};
use lariv_plugin_users::middleware::RequireAuth;

async fn page_from(
    db: &DatabaseConnection,
    prefs: &MdBookPreferences,
    error: String,
    status: String,
) -> MdBookPreferencesPage {
    MdBookPreferencesPage {
        source_vnode_id: id_text(prefs.source_vnode_id),
        source_display: directory_label(db, prefs.source_vnode_id).await,
        output_vnode_id: id_text(prefs.output_vnode_id),
        output_display: directory_label(db, prefs.output_vnode_id).await,
        url_prefix: prefs.url_prefix.clone(),
        build_on_serve: prefs.build_on_serve,
        error,
        status,
    }
}

async fn submitted_page(
    db: &DatabaseConnection,
    form: &PreferencesForm,
    error: String,
) -> MdBookPreferencesPage {
    let source_vnode_id = form.source_vnode_id.trim().parse::<i64>().unwrap_or(0);
    let output_vnode_id = form.output_vnode_id.trim().parse::<i64>().unwrap_or(0);
    MdBookPreferencesPage {
        source_vnode_id: form.source_vnode_id.clone(),
        source_display: directory_label(db, source_vnode_id).await,
        output_vnode_id: form.output_vnode_id.clone(),
        output_display: directory_label(db, output_vnode_id).await,
        url_prefix: form.url_prefix.clone(),
        build_on_serve: checkbox_on(&form.build_on_serve),
        error,
        status: String::new(),
    }
}

async fn parse_form(
    db: &DatabaseConnection,
    form: &PreferencesForm,
) -> Result<MdBookPreferences, String> {
    let source_vnode_id = parse_vnode_id(&form.source_vnode_id, "source directory")?;
    let output_vnode_id = parse_vnode_id(&form.output_vnode_id, "output directory")?;
    let source = require_directory(db, source_vnode_id, "source directory").await?;
    let output = require_directory(db, output_vnode_id, "output directory").await?;
    if directories_overlap(db, &source, &output)
        .await
        .map_err(|error| error.to_string())?
    {
        return Err("source directory and output directory must be different".into());
    }
    let url_prefix = normalize_prefix(&form.url_prefix)?;
    let mut prefs = crate::preferences::default_preferences();
    prefs.source_vnode_id = source.id;
    prefs.output_vnode_id = output.id;
    prefs.url_prefix = url_prefix;
    prefs.build_on_serve = checkbox_on(&form.build_on_serve);
    Ok(prefs)
}

async fn require_directory(db: &DatabaseConnection, id: i64, label: &str) -> Result<VNode, String> {
    let node = node::get_by_id(db, id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("{label} was not found"))?;
    if !node.is_directory {
        return Err(format!("{label} must be a directory"));
    }
    Ok(node)
}

async fn directory_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    match node::get_by_id(db, id).await {
        Ok(Some(node)) => node::get_path(db, &node).await,
        _ => format!("#{id}"),
    }
}

/// `GET /dashboard/mdbook/preferences`
pub async fn get(
    Cap(state): Cap<MdBookState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
) -> Response {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let prefs = match load_preferences(&state.db, state.store.as_ref()).await {
        Ok(prefs) => prefs,
        Err(error) => {
            let page = page_from(
                &state.db,
                &crate::preferences::default_preferences(),
                error.to_string(),
                String::new(),
            )
            .await;
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response();
        }
    };
    let page = page_from(&state.db, &prefs, String::new(), String::new()).await;
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}

/// `POST /dashboard/mdbook/preferences`
pub async fn post(
    Cap(state): Cap<MdBookState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<PreferencesForm>,
) -> Response {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let prefs = match parse_form(&state.db, &form).await {
        Ok(prefs) => prefs,
        Err(error) => {
            let page = submitted_page(&state.db, &form, error).await;
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response();
        }
    };
    match save_preferences(&state.db, state.store.as_ref(), &prefs).await {
        Ok(saved) => {
            let page = page_from(
                &state.db,
                &saved,
                String::new(),
                "Preferences saved.".into(),
            )
            .await;
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
        }
        Err(error) => {
            let page = page_from(&state.db, &prefs, error.to_string(), String::new()).await;
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
        }
    }
}

/// `POST /dashboard/mdbook/build` — publish the saved preferences.
pub async fn build(
    Cap(state): Cap<MdBookState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(_form): HtmlFormBody<BuildForm>,
) -> Response {
    let slot_ctx = SlotCtx::from_auth(&ctx);
    let prefs = match load_preferences(&state.db, state.store.as_ref()).await {
        Ok(prefs) => prefs,
        Err(error) => {
            let page = page_from(
                &state.db,
                &crate::preferences::default_preferences(),
                error.to_string(),
                String::new(),
            )
            .await;
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response();
        }
    };
    let (error, status) = match publish(&state.db, state.store.as_ref(), &prefs).await {
        Ok(PublishOutcome::MissingSource) => (
            format!("source directory #{} was not found", prefs.source_vnode_id),
            String::new(),
        ),
        Ok(PublishOutcome::MissingSummary { source, relative }) => (
            format!("source directory \"{source}\" has no {relative}"),
            String::new(),
        ),
        Ok(PublishOutcome::Published { files, prefix }) => (
            String::new(),
            format!("Published {files} files at {prefix}."),
        ),
        Err(error) => (error.to_string(), String::new()),
    };
    let page = page_from(&state.db, &prefs, error, status).await;
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx).into_response()
}
