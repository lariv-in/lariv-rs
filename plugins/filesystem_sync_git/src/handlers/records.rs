use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::sea_query::Condition;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};

use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::list_filters::{FilterField, apply_filters, clauses_from_query, text_column};
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    path_and_query, respond_create_modal_done_fk, respond_edit_modal_done,
};
use lariv_plugin_filesystem::node;
use lariv_plugin_users::middleware::RequireAuth;
use lariv_plugin_users::role_authorization::scope_allowed;

use crate::entities::filesystem_git_sync::{self, Entity as GitSyncEntity};
use crate::forms::GitSyncForm;
use crate::handlers::ModalNameQuery;
use crate::keys::{
    GIT_SYNC_CREATE_MODAL, GIT_SYNC_DELETE_MODAL, GIT_SYNC_EDIT_MODAL, GIT_SYNC_TABLE,
};
use crate::routes::{FilesystemSyncGitAdmin, GitSyncDetailRouteTag, GitSyncListRouteTag};
use crate::state::FilesystemSyncGitState;
use crate::templates::{
    GitSyncConfirmDeletePage, GitSyncCreateModalPage, GitSyncDetailPage, GitSyncEditModalPage,
    GitSyncListPage, GitSyncRow,
};
use crate::validate::{directory_label, parse_sync_form};

#[derive(Debug, serde::Deserialize, Default)]
pub struct ListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

fn list_url() -> String {
    GitSyncListRouteTag.url()
}

fn fields() -> Vec<FilterField> {
    vec![
        text_column("git_url", "URL", filesystem_git_sync::Column::GitUrl),
        text_column(
            "git_identifier",
            "Git identifier",
            filesystem_git_sync::Column::GitIdentifier,
        ),
    ]
}

fn apply_sort(
    query: sea_orm::Select<GitSyncEntity>,
    sort: Option<&str>,
) -> sea_orm::Select<GitSyncEntity> {
    match sort.unwrap_or("").trim() {
        "URL DESC" => query.order_by_desc(filesystem_git_sync::Column::GitUrl),
        "Identifier DESC" => query.order_by_desc(filesystem_git_sync::Column::GitIdentifier),
        "Identifier" => query.order_by_asc(filesystem_git_sync::Column::GitIdentifier),
        _ => query.order_by_asc(filesystem_git_sync::Column::GitUrl),
    }
}

async fn load_rows(
    db: &sea_orm::DatabaseConnection,
    q: &ListQuery,
    page_size: u32,
    condition: Condition,
) -> ObjectList<GitSyncRow> {
    let mut query = scope_allowed::<FilesystemSyncGitAdmin, _>(GitSyncEntity::find());
    query = query.filter(condition);
    query = apply_sort(query, q.sort.as_deref());
    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(models.len());
    for row in models {
        let directory = directory_label(db, &row.directory_vnode_id.to_string()).await;
        rows.push(GitSyncRow {
            id: row.directory_vnode_id,
            git_url: row.git_url,
            git_identifier: row.git_identifier,
            last_synced: row
                .last_synced_commit_hash
                .filter(|hash| !hash.is_empty())
                .unwrap_or_else(|| "(never synced)".to_string()),
            subpath: row.subpath,
            directory,
        });
    }
    ObjectList::from_page(rows, page, page_size, total)
}

async fn find_row(db: &sea_orm::DatabaseConnection, id: i64) -> Option<filesystem_git_sync::Model> {
    scope_allowed::<FilesystemSyncGitAdmin, _>(GitSyncEntity::find_by_id(id))
        .one(db)
        .await
        .ok()
        .flatten()
}

pub async fn list(
    Cap(state): Cap<FilesystemSyncGitState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ListQuery>,
) -> maud::Markup {
    let clauses = clauses_from_query(uri.query().unwrap_or(""), None);
    let fields = fields();
    let applied = apply_filters(state.db.get_database_backend(), &fields, &clauses);
    let rows = load_rows(&state.db, &q, q.page_size.get(), applied.condition).await;
    let page = GitSyncListPage {
        rows,
        fields,
        filters: FilterClauseView::from_clauses(&clauses, &applied.errors),
        filter_errors: applied.errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets(GIT_SYNC_TABLE) {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn detail(
    Cap(state): Cap<FilesystemSyncGitState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(directory) = node::get_by_id(&state.db, id)
        .await
        .ok()
        .flatten()
        .filter(|node| node.is_directory)
    else {
        return Redirect::to(&list_url()).into_response();
    };
    let page = match find_row(&state.db, id).await {
        Some(row) => GitSyncDetailPage {
            id: row.directory_vnode_id,
            configured: true,
            git_url: row.git_url,
            git_identifier: row.git_identifier,
            last_synced: row
                .last_synced_commit_hash
                .filter(|hash| !hash.is_empty())
                .unwrap_or_else(|| "(never synced)".to_string()),
            subpath: row.subpath,
            directory: directory.name,
        },
        None => GitSyncDetailPage {
            id,
            configured: false,
            git_url: String::new(),
            git_identifier: String::new(),
            last_synced: "(never synced)".to_string(),
            subpath: String::new(),
            directory: directory.name,
        },
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

fn create_page(
    git_url: String,
    git_identifier: String,
    subpath: String,
    directory_vnode_id: String,
    q: &ModalNameQuery,
    directory: String,
    error: String,
) -> GitSyncCreateModalPage {
    GitSyncCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        git_url,
        git_identifier,
        subpath,
        directory_vnode_id,
        directory,
        error,
    }
}

pub async fn create_for_get(
    Cap(state): Cap<FilesystemSyncGitState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(directory_id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let directory = directory_label(&state.db, &directory_id.to_string()).await;
    let page = create_page(
        String::new(),
        String::new(),
        String::new(),
        directory_id.to_string(),
        &q,
        directory,
        String::new(),
    );
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = create_page(
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        &q,
        String::new(),
        String::new(),
    );
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<FilesystemSyncGitState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<GitSyncForm>,
) -> Response {
    let directory = directory_label(&state.db, &form.directory_vnode_id).await;
    let parsed = match parse_sync_form(&state.db, &form, None).await {
        Ok(parsed) => parsed,
        Err(error) => {
            let page = create_page(
                form.git_url,
                form.git_identifier,
                form.subpath,
                form.directory_vnode_id,
                &q,
                directory,
                error,
            );
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let now = Utc::now();
    let model = filesystem_git_sync::ActiveModel {
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        git_url: Set(parsed.git_url),
        git_identifier: Set(parsed.git_identifier),
        last_synced_commit_hash: Set(None),
        subpath: Set(parsed.subpath),
        directory_vnode_id: Set(parsed.directory_vnode_id),
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done_fk(
            &htmx,
            GIT_SYNC_CREATE_MODAL,
            &q.refresh_table(),
            &GitSyncDetailRouteTag::new(saved.directory_vnode_id).url(),
            saved.directory_vnode_id,
            &directory,
            &q.target_input(),
        ),
        Err(error) => {
            let page = create_page(
                form.git_url,
                form.git_identifier,
                form.subpath,
                form.directory_vnode_id,
                &q,
                directory,
                error.to_string(),
            );
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn edit_get(
    Cap(state): Cap<FilesystemSyncGitState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(row) = find_row(&state.db, id).await else {
        return Redirect::to(&list_url()).into_response();
    };
    let page = GitSyncEditModalPage {
        id: row.directory_vnode_id,
        form_name: q.form_name(),
        git_url: row.git_url,
        git_identifier: row.git_identifier,
        subpath: row.subpath,
        directory_vnode_id: row.directory_vnode_id.to_string(),
        directory: directory_label(&state.db, &row.directory_vnode_id.to_string()).await,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<FilesystemSyncGitState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<GitSyncForm>,
) -> Response {
    let Some(existing) = find_row(&state.db, id).await else {
        return Redirect::to(&list_url()).into_response();
    };
    let directory = directory_label(&state.db, &form.directory_vnode_id).await;
    let parsed = match parse_sync_form(&state.db, &form, Some(id)).await {
        Ok(parsed) => parsed,
        Err(error) => {
            let page = GitSyncEditModalPage {
                id,
                form_name: q.form_name(),
                git_url: form.git_url,
                git_identifier: form.git_identifier,
                subpath: form.subpath,
                directory_vnode_id: form.directory_vnode_id,
                directory,
                error,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let mut am: filesystem_git_sync::ActiveModel = existing.into();
    am.updated_at = Set(Some(Utc::now()));
    am.git_url = Set(parsed.git_url);
    am.git_identifier = Set(parsed.git_identifier);
    am.subpath = Set(parsed.subpath);
    am.directory_vnode_id = Set(parsed.directory_vnode_id);
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done(
            &htmx,
            GIT_SYNC_EDIT_MODAL,
            &GitSyncDetailRouteTag::new(id).url(),
        ),
        Err(error) => {
            let page = GitSyncEditModalPage {
                id,
                form_name: q.form_name(),
                git_url: form.git_url,
                git_identifier: form.git_identifier,
                subpath: form.subpath,
                directory_vnode_id: form.directory_vnode_id,
                directory,
                error: error.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = GitSyncConfirmDeletePage {
        modal_uid: GIT_SYNC_DELETE_MODAL.id().to_string(),
        message: "Are you sure you want to delete this git sync?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_filesystem_sync_git.GitSyncDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<FilesystemSyncGitState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match GitSyncEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&list_url()),
        Err(error) => {
            let page = GitSyncConfirmDeletePage {
                modal_uid: GIT_SYNC_DELETE_MODAL.id().to_string(),
                message: "Are you sure you want to delete this git sync?".into(),
                form_name: "p_filesystem_sync_git.GitSyncDeleteForm".into(),
                id,
                error: error.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
