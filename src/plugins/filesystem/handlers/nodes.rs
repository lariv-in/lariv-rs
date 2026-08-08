use axum::{
    Form,
    body::Body,
    extract::{Multipart, Path, Query},
    http::{HeaderValue, StatusCode, Uri, header},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::Deserialize;
use tokio::io::AsyncReadExt;

use crate::{
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    html_form::HtmlForm,
    http::{Cap},
    plugins::{
        filesystem::{
            entities::{
                VNode,
                filesystem_node::{Column, Entity as VNodeEntity},
            },
            forms::{VNodeEditForm, VNodeForm, VNodeKindSubmit, VNodeMultiUploadForm, VNodeZipUploadForm},
            keys::{
                VNodeCreateModalKey, VNodeDeleteModalKey, VNodeMultiUploadModalKey,
                VNodeSelectTableKey, VNodeTableKey, VNodeZipUploadModalKey,
            },
            node,
            routes::{VNodeBrowseRouteTag, VNodeDetailRouteTag, VNodeListRouteTag},
            state::FilesystemState,
            storage::DynFilestore,
            templates::{
                VNodeConfirmDeletePage, VNodeCreateModalPage, VNodeDetailPage, VNodeFormPage,
                VNodeListPage, VNodeMoveFormPage, VNodeMultiUploadModalPage, VNodeOption,
                VNodeRow, VNodeSelectPage, VNodeZipUploadModalPage,
            },
            zip,
        },
        users::{
            middleware::RequireAuth,
            state::AuthContext,
        },
    },
    web::{Htmx, QueryI64, html_built_page_or_app_layout, html_built_page_with_slots, respond_create_modal_done},
};

use super::ModalNameQuery;

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn format_updated_at(dt: Option<chrono::DateTime<Utc>>, tz: &str) -> String {
    crate::datetime::DatetimeLabel::short_optional(dt, tz).into_string()
}

fn slot_ctx(ctx: &AuthContext) -> SlotCtx {
    SlotCtx::from_auth(ctx)
}





// ---------------------------------------------------------------------------
// List / browse
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
pub struct VNodeListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
}

async fn query_nodes(
    db: &DatabaseConnection,
    parent_id: Option<i64>,
    q: &VNodeListQuery,
) -> (Vec<VNode>, u32, u64) {
    let mut query = VNodeEntity::find();
    query = match parent_id {
        Some(id) => query.filter(Column::ParentId.eq(id)),
        None => query.filter(Column::ParentId.is_null()),
    };
    let name = q.name.clone().unwrap_or_default();
    if !name.is_empty() {
        query = query.filter(Column::Name.contains(&name));
    }
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let query = if sort.eq_ignore_ascii_case("Name DESC") {
        query
            .order_by_desc(Column::IsDirectory)
            .order_by_desc(Column::Name)
    } else {
        query
            .order_by_desc(Column::IsDirectory)
            .order_by_asc(Column::Name)
    };

    let page = q.page.unwrap_or(1).max(1);
    let paginator = query.paginate(db, PAGE_SIZE as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

async fn load_list_page(
    db: &DatabaseConnection,
    store: &DynFilestore,
    parent_id: Option<i64>,
    q: &VNodeListQuery,
    tz: &str,
) -> ObjectList<VNodeRow> {
    let (models, page, total) = query_nodes(db, parent_id, q).await;
    let mut rows = Vec::with_capacity(models.len());
    for n in models {
        let size_display = node::file_size_display(store, &n).await;
        let items_display = if n.is_directory {
            node::children_count(db, n.id).await.unwrap_or(0).to_string()
        } else {
            "-".to_string()
        };
        rows.push(VNodeRow {
            id: n.id,
            name: n.name.clone(),
            is_directory: n.is_directory,
            size_display,
            items_display,
            updated_at: format_updated_at(n.updated_at, tz),
        });
    }
    ObjectList::from_page(rows, page, PAGE_SIZE, total)
}

async fn render_list_layered(
    state: FilesystemState,
    auth: AuthContext,
    chrome: SharedChromeFolder,
    htmx: Htmx,
    uri: Uri,
    q: VNodeListQuery,
    parent_id: Option<i64>,
) -> Response
{
    // Equivalent to vnode_list/browse_layers(); avoid run_layers in Route handlers (rustc #100013).
    use crate::plugins::filesystem::layers::VNodeListData;
    let parent = match parent_id {
        Some(id) => match node::get_by_id(&state.db, id).await.ok().flatten() {
            Some(n) if n.is_directory => Some(n),
            _ => return Redirect::to("/filesystem").into_response(),
        },
        None => None,
    };
    let items = load_list_page(&state.db, state.store.as_ref(), parent_id, &q, &auth.timezone).await;
    let list_page = VNodeListPage {
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        parent_name: parent.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        items,
        filter_name: q.name.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    let _ = VNodeListData {
        parent_id: list_page.parent_id,
        parent_name: list_page.parent_name.clone(),
        items: list_page.items.clone(),
        filter_name: list_page.filter_name.clone(),
        sort: list_page.sort.clone(),
        path_and_query: list_page.path_and_query.clone(),
    };
    if htmx.targets::<VNodeTableKey>() {
        return list_page.render_table().into_response();
    }
    html_built_page_or_app_layout(&list_page, &htmx,
        &chrome,
        &slot_ctx(&auth),
    )
    .into_response()
}

/// HTTP handler: `list`.
pub async fn list(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeListQuery>,
) -> Response
{
    render_list_layered(state, auth, chrome, htmx, uri, q, None).await
}

/// HTTP handler: `browse`.
pub async fn browse(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeListQuery>,
    Path(parent_id): Path<i64>,
) -> Response
{
    render_list_layered(state, auth, chrome, htmx, uri, q, Some(parent_id)).await
}

// ---------------------------------------------------------------------------
// Detail
/// ---------------------------------------------------------------------------

/// HTTP handler: `detail`.
pub async fn detail(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
{
    // Layer stack `vnode_detail_layers()` is equivalent; run_layers inside Route::get
    // hits rustc #100013, so execute the detail loader directly here.
    use crate::layers::LoadById;
    use crate::plugins::filesystem::layers::VNodeDetailLoader;
    let Some(data) = VNodeDetailLoader::load_by_id(&state, id).await else {
        return Redirect::to("/filesystem").into_response();
    };
    let detail = VNodeDetailPage {
        id: data.node.id,
        name: data.node.name.clone(),
        is_directory: data.node.is_directory,
        item_type: node::item_type(&data.node).to_string(),
        size_display: data.size_display,
        items_display: data.items_display,
        path: data.path,
        updated_at: format_updated_at(data.node.updated_at, &auth.timezone),
    };
    html_built_page_or_app_layout(&detail, &htmx,
        &chrome,
        &slot_ctx(&auth),
    )
    .into_response()
}

// ---------------------------------------------------------------------------
// Create / edit form
// ---------------------------------------------------------------------------


async fn render_create_get(
    state: FilesystemState,
    auth: AuthContext,
    chrome: SharedChromeFolder,
    q: ModalNameQuery,
    parent_id: Option<i64>,
) -> maud::Markup {
    let parent = match parent_id {
        Some(id) => node::get_by_id(&state.db, id).await.ok().flatten(),
        None => None,
    };
    let page = VNodeCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        name: String::new(),
        is_directory: false,
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        parent_display: parent.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx(&auth))
}

/// HTTP handler: `create_get`.
pub async fn create_get(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    render_create_get(state, auth, chrome, q, None).await
}

/// HTTP handler: `create_get_in`.
pub async fn create_get_in(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(parent_id): Path<i64>,
) -> maud::Markup {
    render_create_get(state, auth, chrome, q, Some(parent_id)).await
}

async fn render_create_post(
    state: FilesystemState,
    auth: AuthContext,
    chrome: SharedChromeFolder,
    htmx: Htmx,
    q: ModalNameQuery,
    parent_id_from_route: Option<i64>,
    multipart: Multipart,
) -> Response
{
    let parsed = match VNodeForm::from_multipart(multipart).await {
        Ok(p) => p,
        Err(e) => {
            return render_create_error(
                &state,
                &chrome,
                &auth,
                &q,
                parent_id_from_route,
                String::new(),
                false,
                e.to_string(),
            )
            .await;
        }
    };
    let parent_id = parent_id_from_route.or(parsed.parent_id.filter(|id| *id != 0));
    let parent = match parent_id {
        Some(id) => node::get_by_id(&state.db, id).await.ok().flatten(),
        None => None,
    };
    let (is_directory, file) = match parsed.kind {
        VNodeKindSubmit::Directory => (true, None),
        VNodeKindSubmit::File { file } => (false, Some(node::NodeFile::Upload(file))),
    };
    match node::create(
        &state.db,
        state.store.as_ref(),
        parsed.name.clone(),
        is_directory,
        file,
        parent.as_ref(),
    )
    .await
    {
        Ok(created) => respond_create_modal_done::<VNodeCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &VNodeDetailRouteTag::new(created.id).url(),
        ),
        Err(e) => {
            render_create_error(
                &state,
                &chrome,
                &auth,
                &q,
                parent_id,
                parsed.name,
                is_directory,
                e.to_string(),
            )
            .await
        }
    }
}

async fn render_create_error(
    state: &FilesystemState,
    chrome: &SharedChromeFolder,
    auth: &AuthContext,
    q: &ModalNameQuery,
    parent_id: Option<i64>,
    name: String,
    is_directory: bool,
    error: String,
) -> Response
{
    let parent = match parent_id {
        Some(id) => node::get_by_id(&state.db, id).await.ok().flatten(),
        None => None,
    };
    let page = VNodeCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        name,
        is_directory,
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        parent_display: parent.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &slot_ctx(auth)).into_response()
}

/// HTTP handler: `create_post`.
pub async fn create_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    multipart: Multipart,
) -> Response
{
    render_create_post(state, auth, chrome, htmx, q, None, multipart).await
}

/// HTTP handler: `create_post_in`.
pub async fn create_post_in(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Path(parent_id): Path<i64>,
    multipart: Multipart,
) -> Response
{
    render_create_post(state, auth, chrome, htmx, q, Some(parent_id), multipart).await
}

// ---------------------------------------------------------------------------
// Edit
/// ---------------------------------------------------------------------------

/// HTTP handler: `edit_get`.
pub async fn edit_get(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
{
    use crate::layers::LoadById;
    use crate::plugins::filesystem::layers::VNodeDetailLoader;
    let Some(data) = VNodeDetailLoader::load_by_id(&state, id).await else {
        return Redirect::to("/filesystem").into_response();
    };
    let d = &data;
    let has_file = d.node.file_path.as_deref().is_some_and(|p| !p.is_empty());
    let form = VNodeFormPage {
        id: d.node.id,
        name: d.node.name.clone(),
        is_directory: d.node.is_directory,
        has_file,
        error: String::new(),
    };
    html_built_page_or_app_layout(&form, &htmx,
        &chrome,
        &slot_ctx(&auth),
    )
    .into_response()
}

/// HTTP handler: `edit_post`.
pub async fn edit_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(auth): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    multipart: Multipart,
) -> Response
{
    use crate::layers::LoadById;
    use crate::plugins::filesystem::layers::VNodeDetailLoader;
    let Some(data) = VNodeDetailLoader::load_by_id(&state, id).await else {
        return Redirect::to("/filesystem").into_response();
    };
    let n = data.node;
    let parsed = match VNodeEditForm::from_multipart(multipart).await {
        Ok(v) => v,
        Err(e) => {
            let has_file = n.file_path.as_deref().is_some_and(|p| !p.is_empty());
            let form = VNodeFormPage {
                id: n.id,
                name: n.name.clone(),
                is_directory: n.is_directory,
                has_file,
                error: e.to_string(),
            };
            return html_built_page_or_app_layout(&form, &htmx,
                &chrome,
                &slot_ctx(&auth),
            )
            .into_response();
        }
    };
    let file = parsed.file.map(node::NodeFile::Upload);
    let is_directory = n.is_directory;
    let has_file_before = n.file_path.as_deref().is_some_and(|p| !p.is_empty());
    let name = parsed.name;
    match node::update(&state.db, state.store.as_ref(), n, name.clone(), file).await {
        Ok(_) => htmx.redirect(&VNodeDetailRouteTag::new(id).url()),
        Err(e) => {
            let form = VNodeFormPage {
                id,
                name,
                is_directory,
                has_file: has_file_before,
                error: e.to_string(),
            };
            html_built_page_or_app_layout(&form, &htmx,
                &chrome,
                &slot_ctx(&auth),
            )
            .into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// Delete
/// ---------------------------------------------------------------------------

/// HTTP handler: `delete_get`.
pub async fn delete_get(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup
{
    let node = node::get_by_id(&state.db, id).await.ok().flatten();
    let message = match node {
        Some(n) if n.is_directory => {
            format!("Are you sure you want to delete \"{}\" and everything inside it?", n.name)
        }
        Some(n) => format!("Are you sure you want to delete \"{}\"?", n.name),
        None => "Are you sure you want to delete this item?".to_string(),
    };
    let page = VNodeConfirmDeletePage {
        modal_uid: VNodeDeleteModalKey::ID.to_string(),
        message,
        form_name: q.name
            .clone()
            .unwrap_or_else(|| "p_filesystem.VNodeDeleteForm".into()),
        id,
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx(&ctx))
}

/// HTTP handler: `delete_post`.
pub async fn delete_post(
    Cap(state): Cap<FilesystemState>,
    RequireAuth(_auth): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    use crate::layers::{DeleteEntity, LoadById};
    use crate::plugins::filesystem::layers::{VNodeDeleter, VNodeDetailLoader};
    if let Some(data) = VNodeDetailLoader::load_by_id(&state, id).await {
        let _ = VNodeDeleter::delete_model(&state, data).await;
    }
    htmx.redirect("/filesystem")
}

// ---------------------------------------------------------------------------
// Move
/// ---------------------------------------------------------------------------

/// HTTP handler: `move_get`.
pub async fn move_get(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response
{
    let Some(n) = node::get_by_id(&state.db, id).await.ok().flatten() else {
        return Redirect::to("/filesystem").into_response();
    };
    let page = VNodeMoveFormPage {
        id: n.id,
        name: n.name,
        is_directory: n.is_directory,
        destination_id: 0,
        destination_display: String::new(),
        error: String::new(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx)).into_response()
}

use crate::plugins::filesystem::forms::MoveForm;

/// HTTP handler: `move_post`.
pub async fn move_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Form(form): Form<MoveForm>,
) -> Response
{
    let Some(n) = node::get_by_id(&state.db, id).await.ok().flatten() else {
        return Redirect::to("/filesystem").into_response();
    };
    let destination = if form.destination_id == 0 {
        None
    } else {
        node::get_by_id(&state.db, form.destination_id).await.ok().flatten()
    };
    let name = n.name.clone();
    let is_directory = n.is_directory;
    match node::move_to(&state.db, n, destination.as_ref()).await {
        Ok(_) => htmx.redirect(&VNodeDetailRouteTag::new(id).url()),
        Err(e) => {
            let destination_display = destination.map(|d| d.name).unwrap_or_default();
            let page = VNodeMoveFormPage {
                id,
                name,
                is_directory,
                destination_id: form.destination_id,
                destination_display,
                error: e.to_string(),
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &slot_ctx(&ctx)).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// Multi-file upload
// ---------------------------------------------------------------------------

async fn render_multi_upload_get(
    state: FilesystemState,
    chrome: SharedChromeFolder,
    ctx: AuthContext,
    q: ModalNameQuery,
    parent_id: Option<i64>,
) -> maud::Markup {
    let parent = match parent_id {
        Some(id) => node::get_by_id(&state.db, id).await.ok().flatten(),
        None => None,
    };
    let page = VNodeMultiUploadModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        parent_display: parent.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx(&ctx))
}

async fn render_multi_upload_error(
    state: &FilesystemState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    q: &ModalNameQuery,
    parent_id: Option<i64>,
    error: String,
) -> Response
{
    let parent = match parent_id {
        Some(id) => node::get_by_id(&state.db, id).await.ok().flatten(),
        None => None,
    };
    let page = VNodeMultiUploadModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        parent_display: parent.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &slot_ctx(ctx)).into_response()
}

async fn render_multi_upload_post(
    state: FilesystemState,
    chrome: SharedChromeFolder,
    ctx: AuthContext,
    htmx: Htmx,
    q: ModalNameQuery,
    parent_id_from_route: Option<i64>,
    multipart: Multipart,
) -> Response
{
    let parsed = match VNodeMultiUploadForm::from_multipart(multipart).await {
        Ok(p) => p,
        Err(e) => {
            return render_multi_upload_error(
                &state,
                &chrome,
                &ctx,
                &q,
                parent_id_from_route,
                e.to_string(),
            )
            .await;
        }
    };
    let parent_id = parent_id_from_route.or(parsed.parent_id.filter(|id| *id != 0));
    if parsed.files.is_empty() {
        return render_multi_upload_error(
            &state,
            &chrome,
            &ctx,
            &q,
            parent_id,
            "please choose at least one file".into(),
        )
        .await;
    }
    let parent = match parent_id {
        Some(id) => node::get_by_id(&state.db, id).await.ok().flatten(),
        None => None,
    };
    let mut first_error = None;
    for file in parsed.files {
        let filename = file.filename().to_string();
        if let Err(e) = node::create(
            &state.db,
            state.store.as_ref(),
            filename,
            false,
            Some(node::NodeFile::Upload(file)),
            parent.as_ref(),
        )
        .await
        {
            first_error.get_or_insert_with(|| e.to_string());
        }
    }
    if let Some(err) = first_error {
        return render_multi_upload_error(&state, &chrome, &ctx, &q, parent_id, err).await;
    }
    let redirect_url = match parent_id {
        Some(id) => VNodeBrowseRouteTag::new(id).url(),
        None => VNodeListRouteTag.url(),
    };
    respond_create_modal_done::<VNodeMultiUploadModalKey>(&htmx, &q.refresh_table(), &redirect_url)
}

/// HTTP handler: `upload_get`.
pub async fn upload_get(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    render_multi_upload_get(state, chrome, ctx, q, None).await
}

/// HTTP handler: `upload_get_in`.
pub async fn upload_get_in(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(parent_id): Path<i64>,
) -> maud::Markup {
    render_multi_upload_get(state, chrome, ctx, q, Some(parent_id)).await
}

/// HTTP handler: `upload_post`.
pub async fn upload_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    multipart: Multipart,
) -> Response
{
    render_multi_upload_post(state, chrome, ctx, htmx, q, None, multipart).await
}

/// HTTP handler: `upload_post_in`.
pub async fn upload_post_in(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Path(parent_id): Path<i64>,
    multipart: Multipart,
) -> Response
{
    render_multi_upload_post(state, chrome, ctx, htmx, q, Some(parent_id), multipart).await
}

// ---------------------------------------------------------------------------
// Zip upload
// ---------------------------------------------------------------------------

async fn render_zip_upload_get(
    state: FilesystemState,
    chrome: SharedChromeFolder,
    ctx: AuthContext,
    q: ModalNameQuery,
    parent_id: Option<i64>,
) -> maud::Markup {
    let parent = match parent_id {
        Some(id) => node::get_by_id(&state.db, id).await.ok().flatten(),
        None => None,
    };
    let page = VNodeZipUploadModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        parent_display: parent.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &slot_ctx(&ctx))
}

async fn render_zip_upload_error(
    state: &FilesystemState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    q: &ModalNameQuery,
    parent_id: Option<i64>,
    error: String,
) -> Response
{
    let parent = match parent_id {
        Some(id) => node::get_by_id(&state.db, id).await.ok().flatten(),
        None => None,
    };
    let page = VNodeZipUploadModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        parent_display: parent.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &slot_ctx(ctx)).into_response()
}

async fn render_zip_upload_post(
    state: FilesystemState,
    chrome: SharedChromeFolder,
    ctx: AuthContext,
    htmx: Htmx,
    q: ModalNameQuery,
    parent_id_from_route: Option<i64>,
    multipart: Multipart,
) -> Response
{
    let parsed = match VNodeZipUploadForm::from_multipart(multipart).await {
        Ok(p) => p,
        Err(e) => {
            return render_zip_upload_error(
                &state,
                &chrome,
                &ctx,
                &q,
                parent_id_from_route,
                e.to_string(),
            )
            .await;
        }
    };
    let parent_id = parent_id_from_route.or(parsed.parent_id.filter(|id| *id != 0));
    let zip_bytes = match parsed.zip_file.into_bytes().await {
        Ok(b) => b,
        Err(e) => {
            return render_zip_upload_error(
                &state,
                &chrome,
                &ctx,
                &q,
                parent_id,
                e.to_string(),
            )
            .await;
        }
    };
    let parent = match parent_id {
        Some(id) => node::get_by_id(&state.db, id).await.ok().flatten(),
        None => None,
    };
    match zip::replace_children_from_zip(&state.db, state.store.as_ref(), parent.as_ref(), &zip_bytes).await {
        Ok(()) => {
            let redirect_url = match parent_id {
                Some(id) => VNodeBrowseRouteTag::new(id).url(),
                None => VNodeListRouteTag.url(),
            };
            respond_create_modal_done::<VNodeZipUploadModalKey>(
                &htmx,
                &q.refresh_table(),
                &redirect_url,
            )
        }
        Err(e) => render_zip_upload_error(&state, &chrome, &ctx, &q, parent_id, e.to_string()).await,
    }
}

/// HTTP handler: `zip_upload_get`.
pub async fn zip_upload_get(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    render_zip_upload_get(state, chrome, ctx, q, None).await
}

/// HTTP handler: `zip_upload_get_in`.
pub async fn zip_upload_get_in(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(parent_id): Path<i64>,
) -> maud::Markup {
    render_zip_upload_get(state, chrome, ctx, q, Some(parent_id)).await
}

/// HTTP handler: `zip_upload_post`.
pub async fn zip_upload_post(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    multipart: Multipart,
) -> Response
{
    render_zip_upload_post(state, chrome, ctx, htmx, q, None, multipart).await
}

/// HTTP handler: `zip_upload_post_in`.
pub async fn zip_upload_post_in(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Path(parent_id): Path<i64>,
    multipart: Multipart,
) -> Response
{
    render_zip_upload_post(state, chrome, ctx, htmx, q, Some(parent_id), multipart).await
}

// ---------------------------------------------------------------------------
// Download
// ---------------------------------------------------------------------------

fn zip_response(filename: &str, bytes: Vec<u8>) -> Response {
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("application/zip"));
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\"")) {
        response.headers_mut().insert(header::CONTENT_DISPOSITION, v);
    }
    response
}

async fn stream_file(state: &FilesystemState, n: &VNode) -> Response {
    let Some(path) = n.file_path.as_deref().filter(|p| !p.is_empty()) else {
        return (StatusCode::NOT_FOUND, "file missing").into_response();
    };
    match state.store.open(path, &n.name).await {
        Ok(mut download) => {
            let mut buf = Vec::new();
            if let Err(e) = download.reader.read_to_end(&mut buf).await {
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }
            let mut response = Response::new(Body::from(buf));
            if let Ok(v) = HeaderValue::from_str(&download.content_type) {
                response.headers_mut().insert(header::CONTENT_TYPE, v);
            }
            if let Ok(v) =
                HeaderValue::from_str(&format!("attachment; filename=\"{}\"", download.filename))
            {
                response.headers_mut().insert(header::CONTENT_DISPOSITION, v);
            }
            response
        }
        Err(e) if e.is_missing() => (StatusCode::NOT_FOUND, "file missing").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// HTTP handler: `download`.
pub async fn download(
    Cap(state): Cap<FilesystemState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    let Some(n) = node::get_by_id(&state.db, id).await.ok().flatten() else {
        return Redirect::to("/filesystem").into_response();
    };
    if n.is_directory {
        match zip::build_zip(&state.db, state.store.as_ref(), Some(&n)).await {
            Ok((filename, bytes)) => zip_response(&filename, bytes),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    } else {
        stream_file(&state, &n).await
    }
}

/// HTTP handler: `download_root`.
pub async fn download_root(Cap(state): Cap<FilesystemState>, RequireAuth(_ctx): RequireAuth) -> Response {
    match zip::build_zip(&state.db, state.store.as_ref(), None).await {
        Ok((filename, bytes)) => zip_response(&filename, bytes),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ---------------------------------------------------------------------------
// Directory picker (Parent / Destination select)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
pub struct VNodeSelectQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub target_input: Option<String>,
    #[serde(default)]
    pub exclude_id: QueryI64,
}

#[allow(clippy::too_many_arguments, reason = "internal fan-in for select route handlers")]
async fn render_select(
    state: FilesystemState,
    chrome: SharedChromeFolder,
    ctx: AuthContext,
    htmx: Htmx,
    uri: Uri,
    q: VNodeSelectQuery,
    parent_id: Option<i64>,
    browse_base: &str,
    default_target_input: &str,
    only_directories: bool,
) -> Response
{
    let parent = match parent_id {
        Some(id) => node::get_by_id(&state.db, id).await.ok().flatten(),
        None => None,
    };
    let name_filter = q.name.clone().unwrap_or_default();
    let mut children = node::list_children(&state.db, parent_id, only_directories, &name_filter)
        .await
        .unwrap_or_default();
    children.sort_by(|a, b| a.name.cmp(&b.name));
    let total = children.len() as u64;
    let options: Vec<VNodeOption> = children
        .into_iter()
        .map(|n| VNodeOption { id: n.id, name: n.name })
        .collect();
    let page_size = (total.max(1) as u32).min(500);
    let items = ObjectList::from_page(options, 1, page_size, total);
    let current_path = match &parent {
        Some(p) => node::get_path(&state.db, p).await,
        None => "/".to_string(),
    };
    let page = VNodeSelectPage {
        items,
        filter_name: name_filter,
        target_input: q
            .target_input
            .clone()
            .unwrap_or_else(|| default_target_input.to_string()),
        browse_base: browse_base.to_string(),
        parent_id: parent.as_ref().map(|p| p.id).unwrap_or(0),
        current_path,
        exclude_id: q.exclude_id.or_zero(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    if htmx.targets::<VNodeSelectTableKey>() {
        return page.render_table().into_response();
    }
    html_built_page_with_slots(&page, &chrome, &slot_ctx(&ctx)).into_response()
}

/// HTTP handler: `select`.
pub async fn select(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeSelectQuery>,
) -> Response
{
    render_select(state, chrome, ctx, htmx, uri, q, None, "/filesystem/select", "ParentID", true).await
}

/// HTTP handler: `select_in`.
pub async fn select_in(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeSelectQuery>,
    Path(parent_id): Path<i64>,
) -> Response
{
    render_select(
        state,
        chrome,
        ctx,
        htmx,
        uri,
        q,
        Some(parent_id),
        "/filesystem/select",
        "ParentID",
        true,
    )
    .await
}

/// HTTP handler: `move_select`.
pub async fn move_select(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeSelectQuery>,
) -> Response
{
    render_select(
        state,
        chrome,
        ctx,
        htmx,
        uri,
        q,
        None,
        "/filesystem/move-select",
        "DestinationID",
        true,
    )
    .await
}

/// HTTP handler: `move_select_in`.
pub async fn move_select_in(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeSelectQuery>,
    Path(parent_id): Path<i64>,
) -> Response
{
    render_select(
        state,
        chrome,
        ctx,
        htmx,
        uri,
        q,
        Some(parent_id),
        "/filesystem/move-select",
        "DestinationID",
        true,
    )
    .await
}

/// HTTP handler: `file_select`.
pub async fn file_select(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeSelectQuery>,
) -> Response
{
    render_select(
        state,
        chrome,
        ctx,
        htmx,
        uri,
        q,
        None,
        "/filesystem/file-select",
        "PageID",
        false,
    )
    .await
}

/// HTTP handler: `file_select_in`.
pub async fn file_select_in(
    Cap(state): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<VNodeSelectQuery>,
    Path(parent_id): Path<i64>,
) -> Response
{
    render_select(
        state,
        chrome,
        ctx,
        htmx,
        uri,
        q,
        Some(parent_id),
        "/filesystem/file-select",
        "PageID",
        false,
    )
    .await
}
