use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};

use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_with_slots};
use lariv_plugin_filesystem::node;
use lariv_plugin_filesystem::permissions::AccessActor;
use lariv_plugin_filesystem::routes::VNodeDetailRouteTag;
use lariv_plugin_users::middleware::RequireAuth;

use crate::entities::filesystem_git_sync::Entity as GitSyncEntity;
use crate::entities::filesystem_git_sync::{self, Column};
use crate::fetch::fetch_entries;
use crate::state::FilesystemSyncGitState;
use crate::sync_tree::merge_into;
use crate::templates::GitSyncConfirmPage;
use crate::validate::should_merge;

struct ConfirmExtra {
    force: bool,
    resolved: String,
    notice: String,
    error: String,
}

fn fresh(force: bool) -> ConfirmExtra {
    ConfirmExtra {
        force,
        resolved: String::new(),
        notice: String::new(),
        error: String::new(),
    }
}

pub async fn confirm_get(
    Cap(state): Cap<FilesystemSyncGitState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    Path(directory_id): Path<i64>,
) -> Response {
    show(&state, &chrome, &auth, directory_id, fresh(true)).await
}

pub async fn git_sync_get(
    Cap(state): Cap<FilesystemSyncGitState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    Path(directory_id): Path<i64>,
) -> Response {
    show(&state, &chrome, &auth, directory_id, fresh(false)).await
}

pub async fn confirm_post(
    state: Cap<FilesystemSyncGitState>,
    chrome: Cap<SharedChromeFolder>,
    auth: RequireAuth,
    htmx: Htmx,
    directory_id: Path<i64>,
) -> Response {
    run(state, chrome, auth, htmx, directory_id, true).await
}

pub async fn git_sync_post(
    state: Cap<FilesystemSyncGitState>,
    chrome: Cap<SharedChromeFolder>,
    auth: RequireAuth,
    htmx: Htmx,
    directory_id: Path<i64>,
) -> Response {
    run(state, chrome, auth, htmx, directory_id, false).await
}

async fn show(
    state: &FilesystemSyncGitState,
    chrome: &SharedChromeFolder,
    auth: &lariv_plugin_users::state::AuthContext,
    directory_id: i64,
    extra: ConfirmExtra,
) -> Response {
    match load_confirm(state, auth, directory_id, extra).await {
        Ok(page) => {
            html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(auth)).into_response()
        }
        Err(response) => *response,
    }
}

async fn run(
    Cap(state): Cap<FilesystemSyncGitState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    htmx: Htmx,
    Path(directory_id): Path<i64>,
    force: bool,
) -> Response {
    let page = match load_confirm(&state, &auth, directory_id, fresh(force)).await {
        Ok(page) => page,
        Err(response) => return *response,
    };
    let url = page.git_url.clone();
    let identifier = page.git_identifier.clone();
    let subpath = page.subpath.clone();
    let fetched =
        tokio::task::spawn_blocking(move || fetch_entries(&url, &identifier, &subpath)).await;
    let fetched = match fetched {
        Ok(Ok(fetched)) => fetched,
        Ok(Err(error)) => {
            return show(
                &state,
                &chrome,
                &auth,
                directory_id,
                ConfirmExtra {
                    error,
                    ..fresh(force)
                },
            )
            .await;
        }
        Err(error) => {
            return show(
                &state,
                &chrome,
                &auth,
                directory_id,
                ConfirmExtra {
                    error: error.to_string(),
                    ..fresh(force)
                },
            )
            .await;
        }
    };
    let last_synced = if page.last_synced_commit_hash.is_empty() {
        None
    } else {
        Some(page.last_synced_commit_hash.as_str())
    };
    if !should_merge(force, last_synced, &fetched.commit_id) {
        let notice = format!(
            "Already synced to {}. Nothing was changed.",
            fetched.commit_id
        );
        return show(
            &state,
            &chrome,
            &auth,
            directory_id,
            ConfirmExtra {
                resolved: fetched.commit_id,
                notice,
                ..fresh(force)
            },
        )
        .await;
    }
    let Some(directory) = node::get_by_id(&state.db, directory_id)
        .await
        .ok()
        .flatten()
    else {
        return show(
            &state,
            &chrome,
            &auth,
            directory_id,
            ConfirmExtra {
                resolved: fetched.commit_id,
                error: "directory was not found".into(),
                ..fresh(force)
            },
        )
        .await;
    };
    if let Err(error) = merge_into(
        &state.db,
        state.store.as_ref(),
        &directory,
        &fetched.entries,
        directory.owner_id,
        directory.role.clone(),
    )
    .await
    {
        return show(
            &state,
            &chrome,
            &auth,
            directory_id,
            ConfirmExtra {
                resolved: fetched.commit_id,
                error: error.to_string(),
                ..fresh(force)
            },
        )
        .await;
    }
    let saved = filesystem_git_sync::ActiveModel {
        directory_vnode_id: Set(page.directory_id),
        last_synced_commit_hash: Set(Some(fetched.commit_id.clone())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    };
    if let Err(error) = saved.update(&state.db).await {
        return show(
            &state,
            &chrome,
            &auth,
            directory_id,
            ConfirmExtra {
                resolved: fetched.commit_id,
                error: error.to_string(),
                ..fresh(force)
            },
        )
        .await;
    }
    htmx.redirect(&VNodeDetailRouteTag::new(directory_id).url())
}

async fn load_confirm(
    state: &FilesystemSyncGitState,
    auth: &lariv_plugin_users::state::AuthContext,
    directory_id: i64,
    extra: ConfirmExtra,
) -> Result<GitSyncConfirmPage, Box<Response>> {
    let Some(directory) = node::get_by_id(&state.db, directory_id)
        .await
        .ok()
        .flatten()
        .filter(|node| node.is_directory)
    else {
        return Err(Box::new(
            (StatusCode::NOT_FOUND, "directory was not found").into_response(),
        ));
    };
    let actor = AccessActor::from_auth(auth);
    if let Err(error) = node::authorize_create_in(&state.db, Some(&directory), &actor).await {
        return Err(Box::new(
            (StatusCode::FORBIDDEN, error.to_string()).into_response(),
        ));
    }
    let Some(row) = GitSyncEntity::find()
        .filter(Column::DirectoryVnodeId.eq(directory_id))
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Err(Box::new(
            (
                StatusCode::NOT_FOUND,
                "this directory has no git sync configuration",
            )
                .into_response(),
        ));
    };
    Ok(GitSyncConfirmPage {
        directory_id,
        directory_name: directory.name,
        git_url: row.git_url,
        git_identifier: row.git_identifier,
        subpath: row.subpath,
        last_synced_commit_hash: row.last_synced_commit_hash.unwrap_or_default(),
        resolved_commit: extra.resolved,
        force: extra.force,
        notice: extra.notice,
        error: extra.error,
    })
}
