//! List load layer — paginated, filtered collection into layer Data.
//!
//! Parses common query params (`Name`, `sort`, `page`) and loads an [`ObjectList`] via
//! a plugin-defined [`LoadList`] implementation.
//!
//! # Use cases
//!
//! - Admin index pages with name filter and pagination.
//! - Nested lists scoped by a parent id from the path (`with_path_scope`).
//!
//! # Examples
//!
//! ```rust ignore
//! view::<ClientListPage>()
//!     .layer(ListLayer::<ClientLoader, ClientListTag>::new())
//!
//! // Nested: /clients/{id}/proposals/
//! view::<ProposalListPage>()
//!     .layer(PathLayer::names(&["id"]))
//!     .layer(ListLayer::<ProposalLoader, ProposalListTag>::new().with_path_scope())
//! ```

use std::future::Future;
use std::marker::PhantomData;

use frunk::{HCons, HNil, hlist::HList};

use crate::components::ObjectList;
use crate::layers::{LayerContrib, LayerRequest, LayerStep, ViewLayer, cons_tagged};
use crate::tag::Tagged;

/// Query parameters for list layers (mirrors common Name/sort/page filters for list filters).
#[derive(Clone, Debug, Default)]
pub struct ListQuery {
    pub name: String,
    pub sort: String,
    pub page: u32,
}

impl ListQuery {
    pub fn from_request(req: &LayerRequest) -> Self {
        Self {
            name: req
                .query
                .get("Name")
                .or_else(|| req.query.get("name"))
                .cloned()
                .unwrap_or_default(),
            sort: req.query.get("sort").cloned().unwrap_or_default(),
            page: req
                .query
                .get("page")
                .and_then(|p| p.parse().ok())
                .unwrap_or(1),
        }
    }
}

/// Load a page of models for a list view.
pub trait LoadList: Send + Sync {
    type Model: Clone + Send + Sync + 'static;
    type State: Sync;
    type Scope: Clone + Send + Sync + 'static;

    fn load_list(
        state: &Self::State,
        scope: Option<Self::Scope>,
        query: &ListQuery,
    ) -> impl Future<Output = ObjectList<Self::Model>> + Send;
}

/// Context that exposes list-loader state.
pub trait HasListState<L: LoadList> {
    fn list_state(&self) -> &L::State;
}

/// Context that can supply list scope (e.g. parent id from path).
pub trait HasListScope<L: LoadList> {
    fn list_scope(&self, req: &LayerRequest) -> Option<L::Scope>;
}

/// Loads a paginated [`ObjectList`] and stores it under compile-time tag `Key`.
pub struct ListLayer<Loader, Key>
where
    Loader: LoadList,
{
    /// When `true`, pass [`HasListScope::list_scope`] to the loader (parent id from path).
    pub use_path_scope: bool,
    _loader: PhantomData<fn() -> Loader>,
    _key: PhantomData<fn() -> Key>,
}

impl<Loader, Key> ListLayer<Loader, Key>
where
    Loader: LoadList,
{
    /// List layer without path-based scope filtering.
    pub const fn new() -> Self {
        Self {
            use_path_scope: false,
            _loader: PhantomData,
            _key: PhantomData,
        }
    }

    /// Enable parent-id (or similar) scope from the path via [`HasListScope`].
    pub const fn with_path_scope(self) -> Self {
        Self {
            use_path_scope: true,
            _loader: PhantomData,
            _key: PhantomData,
        }
    }
}

impl<Loader, Key> Default for ListLayer<Loader, Key>
where
    Loader: LoadList,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Loader, Key> Clone for ListLayer<Loader, Key>
where
    Loader: LoadList,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Loader, Key> Copy for ListLayer<Loader, Key> where Loader: LoadList {}

impl<Loader, Key> LayerContrib for ListLayer<Loader, Key>
where
    Loader: LoadList,
{
    type Contrib = HCons<Tagged<Key, ObjectList<Loader::Model>>, HNil>;
}

impl<Ctx, Acc, Loader, Key> ViewLayer<Ctx, Acc> for ListLayer<Loader, Key>
where
    Acc: HList + Send,
    Ctx: HasListState<Loader> + HasListScope<Loader> + Send,
    Loader: LoadList,
    Key: Send + Sync + 'static,
{
    type AccOut = HCons<Tagged<Key, ObjectList<Loader::Model>>, Acc>;

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
                ctx.list_scope(req)
            } else {
                None
            };
            let query = ListQuery::from_request(req);
            let list = Loader::load_list(ctx.list_state(), scope, &query).await;
            LayerStep::Continue(cons_tagged::<Key, _, _>(list, acc))
        }
    }
}
