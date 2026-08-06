//! Filesystem view-layer loaders, run context, and [`BuildFromData`] impls.

use std::collections::HashMap;
use std::future::Future;

use axum::response::IntoResponse;
use chrono::Utc;
use frunk::{HCons, HNil, hlist::HList};

use crate::components::{DEFAULT_PAGE_SIZE, ObjectList};
use crate::layers::{
    BuildFromData, CreateEntity, DeleteEntity, HasCreateState, HasDeleteState,
    HasFormMapsRef, HasLoadState, HasUpdateState, LayerContrib, LayerRequest, LayerStep,
    LoadById, UpdateEntity, ViewLayer, cons_tagged,
};
use crate::plugins::filesystem::{
    entities::VNode,
    node,
    state::FilesystemState,
    templates::{VNodeDetailPage, VNodeFormPage, VNodeListPage, VNodeRow},
};
use crate::plugins::users::layers::AuthSlot;
use crate::plugins::users::state::AuthContext;
use crate::tag::Tagged;

/// Tag for a loaded vnode (detail / form / delete).
pub struct VNodeKey;

/// Tag for vnode list bundle.
pub struct VNodeListKey;

/// Folded Data for detail/edit stacks (detail contrib only; auth is seeded on [`FsViewCtx`]).
pub type DetailData = HCons<Tagged<VNodeKey, VNodeDetailData>, HNil>;

/// Folded Data for list/browse stacks.
pub type ListData = HCons<Tagged<VNodeListKey, VNodeListData>, HNil>;

/// Enriched detail payload (not raw SeaORM model).
#[derive(Clone, Debug)]
pub struct VNodeDetailData {
    pub node: VNode,
    pub size_display: String,
    pub items_display: String,
    pub path: String,
    pub updated_at: String,
}

/// List page payload including filters and parent scope.
#[derive(Clone)]
pub struct VNodeListData {
    pub parent_id: i64,
    pub parent_name: String,
    pub items: ObjectList<VNodeRow>,
    pub filter_name: String,
    pub sort: String,
    pub path_and_query: String,
}

fn format_updated_at(dt: Option<chrono::DateTime<Utc>>, tz: &str) -> String {
    dt.map(|d| crate::datetime::format_datetime_short(d, tz))
        .unwrap_or_default()
}

/// Loader for detail / edit / delete stacks.
pub struct VNodeDetailLoader;

impl LoadById for VNodeDetailLoader {
    type Model = VNodeDetailData;
    type State = FilesystemState;

    async fn load_by_id(state: &Self::State, id: i64) -> Option<Self::Model> {
        let n = node::get_by_id(&state.db, id).await.ok().flatten()?;
        let size_display = node::file_size_display(state.store.as_ref(), &n).await;
        let items_display = if n.is_directory {
            node::children_count(&state.db, n.id)
                .await
                .unwrap_or(0)
                .to_string()
        } else {
            "-".to_string()
        };
        let path = node::get_path(&state.db, &n).await;
        let updated_at = format_updated_at(n.updated_at, crate::datetime::DEFAULT_TIMEZONE);
        Some(VNodeDetailData {
            node: n,
            size_display,
            items_display,
            path,
            updated_at,
        })
    }
}

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

async fn load_list_rows(
    state: &FilesystemState,
    parent_id: Option<i64>,
    name: &str,
    sort: &str,
    page: u32,
    tz: &str,
) -> ObjectList<VNodeRow> {
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

    use crate::plugins::filesystem::entities::filesystem_node::{Column, Entity as VNodeEntity};

    let mut query = VNodeEntity::find().filter(Column::DeletedAt.is_null());
    query = match parent_id {
        Some(id) => query.filter(Column::ParentId.eq(id)),
        None => query.filter(Column::ParentId.is_null()),
    };
    if !name.is_empty() {
        query = query.filter(Column::Name.contains(name));
    }
    let query = if sort.eq_ignore_ascii_case("Name DESC") {
        query
            .order_by_desc(Column::IsDirectory)
            .order_by_desc(Column::Name)
    } else {
        query
            .order_by_desc(Column::IsDirectory)
            .order_by_asc(Column::Name)
    };
    let page = page.max(1);
    let paginator = query.paginate(&state.db, PAGE_SIZE as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let nodes = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(nodes.len());
    for n in nodes {
        let size_display = node::file_size_display(state.store.as_ref(), &n).await;
        let items_display = if n.is_directory {
            node::children_count(&state.db, n.id)
                .await
                .unwrap_or(0)
                .to_string()
        } else {
            "-".to_string()
        };
        rows.push(VNodeRow {
            id: n.id,
            name: n.name,
            is_directory: n.is_directory,
            size_display,
            items_display,
            updated_at: format_updated_at(n.updated_at, tz),
        });
    }
    ObjectList::from_page(rows, page, PAGE_SIZE, total)
}

/// Filesystem list/browse layer — contributes a full [`VNodeListData`].
#[derive(Clone, Copy, Debug)]
pub struct VNodeListBundleLayer {
    pub use_path_scope: bool,
}

impl VNodeListBundleLayer {
    pub const fn root() -> Self {
        Self {
            use_path_scope: false,
        }
    }

    pub const fn browse() -> Self {
        Self {
            use_path_scope: true,
        }
    }
}

impl LayerContrib for VNodeListBundleLayer {
    type Contrib = HCons<Tagged<VNodeListKey, VNodeListData>, HNil>;
}

impl<Ctx, Acc> ViewLayer<Ctx, Acc> for VNodeListBundleLayer
where
    Acc: HList + Send,
    Ctx: HasLoadState<VNodeDetailLoader> + AuthSlot + Send,
{
    type AccOut = HCons<Tagged<VNodeListKey, VNodeListData>, Acc>;

    fn run<'a>(
        &'a self,
        ctx: &'a mut Ctx,
        req: &'a mut LayerRequest,
        acc: Acc,
    ) -> impl Future<Output = LayerStep<Self::AccOut>> + Send + 'a
    where
        Acc: Send + 'a,
    {
        async move {
            let scope = if self.use_path_scope {
                req.path_i64("parent_id")
            } else {
                None
            };
            let state = ctx.load_state().clone();
            let parent = match scope {
                Some(id) => match node::get_by_id(&state.db, id).await.ok().flatten() {
                    Some(n) if n.is_directory => Some(n),
                    _ => {
                        return LayerStep::Done(
                            axum::response::Redirect::to("/filesystem").into_response(),
                        );
                    }
                },
                None => None,
            };
            let parent_id = parent.as_ref().map(|p| p.id).unwrap_or(0);
            let parent_name = parent.as_ref().map(|p| p.name.clone()).unwrap_or_default();
            let name = req
                .query
                .get("Name")
                .or_else(|| req.query.get("name"))
                .cloned()
                .unwrap_or_default();
            let sort = req.query.get("sort").cloned().unwrap_or_default();
            let page = req
                .query
                .get("page")
                .and_then(|p| p.parse().ok())
                .unwrap_or(1);
            let tz = ctx
                .auth()
                .map(|a| a.timezone.as_str())
                .unwrap_or(crate::datetime::DEFAULT_TIMEZONE);
            let items = load_list_rows(&state, scope, &name, &sort, page, tz).await;
            let path_and_query = req
                .uri
                .path_and_query()
                .map(|pq| pq.as_str().to_string())
                .unwrap_or_else(|| req.uri.path().to_string());
            let bundle = VNodeListData {
                parent_id,
                parent_name,
                items,
                filter_name: name,
                sort,
                path_and_query,
            };
            LayerStep::Continue(cons_tagged::<VNodeListKey, _, _>(bundle, acc))
        }
    }
}

/// Update entity adapter (name-only form map; file uploads handled outside the layer).
pub struct VNodeUpdater;

impl UpdateEntity for VNodeUpdater {
    type Model = VNodeDetailData;
    type State = FilesystemState;

    async fn update_from_form(
        state: &Self::State,
        model: Self::Model,
        values: &HashMap<String, String>,
    ) -> Result<Self::Model, String> {
        let name = values
            .get("Name")
            .cloned()
            .unwrap_or_else(|| model.node.name.clone());
        let updated = node::update(&state.db, state.store.as_ref(), model.node, name, None)
            .await
            .map_err(|e| e.to_string())?;
        VNodeDetailLoader::load_by_id(state, updated.id)
            .await
            .ok_or_else(|| "updated node missing".into())
    }

    fn success_url(model: &Self::Model) -> String {
        format!("/filesystem/{}", model.node.id)
    }
}

/// Delete adapter.
pub struct VNodeDeleter;

impl DeleteEntity for VNodeDeleter {
    type Model = VNodeDetailData;
    type State = FilesystemState;

    async fn delete_model(state: &Self::State, model: Self::Model) -> Result<(), String> {
        node::delete_tree(&state.db, state.store.as_ref(), &model.node)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn success_url() -> &'static str {
        "/filesystem"
    }
}

/// Create adapter (metadata-only; binary uploads stay in handlers).
pub struct VNodeCreator;

impl CreateEntity for VNodeCreator {
    type Model = VNode;
    type State = FilesystemState;

    async fn create_from_form(
        state: &Self::State,
        values: &HashMap<String, String>,
    ) -> Result<Self::Model, String> {
        let name = values.get("Name").cloned().unwrap_or_default();
        let is_directory = values
            .get("IsDirectory")
            .map(|v| matches!(v.as_str(), "true" | "on" | "1"))
            .unwrap_or(false);
        let parent_id = values
            .get("ParentID")
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|id| *id != 0);
        let parent = match parent_id {
            Some(id) => node::get_by_id(&state.db, id).await.ok().flatten(),
            None => None,
        };
        node::create(
            &state.db,
            state.store.as_ref(),
            name,
            is_directory,
            None,
            parent.as_ref(),
        )
        .await
        .map_err(|e| e.to_string())
    }

    fn created_id(model: &Self::Model) -> i64 {
        model.id
    }
}

/// Per-request context for filesystem view stacks.
///
/// Auth is seeded from [`RequireAuth`](crate::plugins::users::middleware::RequireAuth) on HTTP
/// handlers (pairing `AuthLayer` + `HeaderMap` extractors with `Route::get` hits rustc #100013).
pub struct FsViewCtx {
    pub fs: FilesystemState,
    pub auth: Option<AuthContext>,
    pub form_values: HashMap<String, String>,
}

impl FsViewCtx {
    pub fn new(fs: FilesystemState) -> Self {
        Self {
            fs,
            auth: None,
            form_values: HashMap::new(),
        }
    }

    pub fn slot_ctx(&self) -> crate::components::SlotCtx {
        self.auth
            .as_ref()
            .map(crate::components::SlotCtx::from_auth)
            .unwrap_or_default()
    }
}

impl AuthSlot for FsViewCtx {
    fn set_auth(&mut self, auth: AuthContext) {
        self.auth = Some(auth);
    }

    fn auth(&self) -> Option<&AuthContext> {
        self.auth.as_ref()
    }
}

impl HasLoadState<VNodeDetailLoader> for FsViewCtx {
    fn load_state(&self) -> &FilesystemState {
        &self.fs
    }
}

impl HasUpdateState<VNodeUpdater> for FsViewCtx {
    fn update_state(&self) -> &FilesystemState {
        &self.fs
    }
}

impl HasDeleteState<VNodeDeleter> for FsViewCtx {
    fn delete_state(&self) -> &FilesystemState {
        &self.fs
    }
}

impl HasCreateState<VNodeCreator> for FsViewCtx {
    fn create_state(&self) -> &FilesystemState {
        &self.fs
    }
}

impl HasFormMapsRef for FsViewCtx {
    fn form_values(&self) -> &HashMap<String, String> {
        &self.form_values
    }
}

impl<Tail> BuildFromData<HCons<Tagged<VNodeKey, VNodeDetailData>, Tail>> for VNodeDetailPage
where
    Tail: HList,
{
    fn build_from_data(data: &HCons<Tagged<VNodeKey, VNodeDetailData>, Tail>) -> Self {
        let d = &data.head.value;
        Self {
            id: d.node.id,
            name: d.node.name.clone(),
            is_directory: d.node.is_directory,
            item_type: node::item_type(&d.node).to_string(),
            size_display: d.size_display.clone(),
            items_display: d.items_display.clone(),
            path: d.path.clone(),
            updated_at: d.updated_at.clone(),
        }
    }
}

impl<Tail> BuildFromData<HCons<Tagged<VNodeKey, VNodeDetailData>, Tail>> for VNodeFormPage
where
    Tail: HList,
{
    fn build_from_data(data: &HCons<Tagged<VNodeKey, VNodeDetailData>, Tail>) -> Self {
        let d = &data.head.value;
        let has_file = d.node.file_path.as_deref().is_some_and(|p| !p.is_empty());
        Self {
            id: d.node.id,
            name: d.node.name.clone(),
            is_directory: d.node.is_directory,
            has_file,
            error: String::new(),
        }
    }
}

impl<Tail> BuildFromData<HCons<Tagged<VNodeListKey, VNodeListData>, Tail>> for VNodeListPage
where
    Tail: HList,
{
    fn build_from_data(data: &HCons<Tagged<VNodeListKey, VNodeListData>, Tail>) -> Self {
        let b = &data.head.value;
        Self {
            parent_id: b.parent_id,
            parent_name: b.parent_name.clone(),
            items: b.items.clone(),
            filter_name: b.filter_name.clone(),
            sort: b.sort.clone(),
            path_and_query: b.path_and_query.clone(),
        }
    }
}
