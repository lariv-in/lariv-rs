//! Typed HTTP routing capability for Lariv apps.
//!
//! Mounts compile-time route HLists onto axum, publishes capability values into request
//! extensions, and wires HTMX middleware. Route identity tags live in [`route_tag`].
//!
//! # Routes
//!
//! Register handlers with [`Route::get`], [`Route::post`], and sibling helpers. Prepend
//! tagged routes onto [`crate::http::HttpCapability`] (usually via plugin [`RouteRegistrar`] hooks).
//! At runtime [`into_axum_router`] folds the mounted route list into an axum [`Router`].
//!
//! # Use cases
//!
//! - Bootstrap an app with [`with_http`] before plugins append routes.
//! - Extract mounted plugin state in handlers via [`Cap`] (reads request extensions).
//! - Build the production router with capability injection and HTMX redirect rewriting.
//!
//! # Examples
//!
//! ```rust
//! # use lariv_rs::http::with_http;
//! # use lariv_rs::app::App;
//! let _app = with_http(App::new());
//! ```
//!
//! ```rust ignore
//! // Handler extracting a database pool published from the mounted App HList.
//! async fn list_users(Cap(db): Cap<Arc<DbPool>>) -> impl IntoResponse { /* ... */ }
//! ```

use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::Request,
    handler::Handler,
    http::Extensions,
    middleware::{self, Next},
    routing::{MethodRouter, delete, get, head, options, patch, post, put, trace},
};
use frunk::{HCons, HNil, hlist::HList};

use crate::{
    app::{App, MountedApp},
    capability::{CapStore, Capability},
    components::slots::{SharedChromeFolder, SlotTag},
    tag::Tagged,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::GetByTag,
    },
};

pub mod route_tag;

pub use route_tag::{
    AppPaneGet, AppPanePost, BoostPost, FileDownloadGet, FileDownloadPost, FragmentGet,
    FragmentPost, GenerationPost, ModalGet, RouteQueryBuilder, RouteTag, RouteUrl,
    trailing_slash,
};

/// Capability tag identifying the HTTP router on the app HList.
///
/// Used with [`GetByTag`] to retrieve
/// [`crate::http::HttpCapability`] after mount.
pub struct HttpTag;

/// HTTP method marker stored on a [`Route`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    Trace,
}

/// A single axum route entry in the HTTP capability's compile-time HList.
///
/// Created via [`Route::get`], [`Route::post`], etc. Paths are normalized (trailing
/// slash stripped except for root).
#[derive(Clone)]
pub struct Route {
    pub path: String,
    pub method: Method,
    method_router: MethodRouter<()>,
}

impl Route {
    fn new(path: impl Into<String>, method: Method, method_router: MethodRouter<()>) -> Self {
        Self {
            path: normalize_route_path(path),
            method,
            method_router,
        }
    }

    /// Register a GET handler at `path`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use lariv_rs::http::Route;
    /// async fn list_users() {}
    /// let _route = Route::get("/users/", list_users);
    /// ```
    pub fn get<H, T>(path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        Self::new(path, Method::Get, get(handler))
    }

    /// Register a POST handler at `path`.
    ///
    /// # Use cases
    ///
    /// - Form submissions (create/update/delete) that swap HTMX regions.
    /// - Non-idempotent actions (logout, generation triggers).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use lariv_rs::http::Route;
    /// async fn create_user() {}
    /// let _route = Route::post("/users/create", create_user);
    /// ```
    pub fn post<H, T>(path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        Self::new(path, Method::Post, post(handler))
    }

    /// Register a PUT handler at `path`.
    pub fn put<H, T>(path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        Self::new(path, Method::Put, put(handler))
    }

    /// Register a DELETE handler at `path`.
    pub fn delete<H, T>(path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        Self::new(path, Method::Delete, delete(handler))
    }

    /// Register a PATCH handler at `path`.
    pub fn patch<H, T>(path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        Self::new(path, Method::Patch, patch(handler))
    }

    /// Register a HEAD handler at `path`.
    pub fn head<H, T>(path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        Self::new(path, Method::Head, head(handler))
    }

    /// Register an OPTIONS handler at `path`.
    pub fn options<H, T>(path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        Self::new(path, Method::Options, options(handler))
    }

    /// Register a TRACE handler at `path`.
    pub fn trace<H, T>(path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        Self::new(path, Method::Trace, trace(handler))
    }
}

fn normalize_route_path(path: impl Into<String>) -> String {
    let path = path.into();
    if path.len() > 1 && path.ends_with('/') {
        path.trim_end_matches('/').to_owned()
    } else {
        path
    }
}

/// Plugin hook for appending routes onto an [`crate::http::HttpCapability`].
pub trait RouteRegistrar<Http, Proof = ()>: Sized {
    type Output;
    fn register_routes(self, http: Http) -> Self::Output;
}

/// Apply queued route hooks (tail first so install order is preserved).
pub trait FoldMountRoutes<Http, Proof = ()>: Sized {
    type Output;
    fn fold_mount_routes(self, http: Http) -> Self::Output;
}

impl<Http> FoldMountRoutes<Http> for HNil {
    type Output = Http;

    fn fold_mount_routes(self, http: Http) -> Self::Output {
        http
    }
}

impl<Plugin, Hook, Tail, Http, TailProof, Proof> FoldMountRoutes<Http, (TailProof, Proof)>
    for HCons<Tagged<Plugin, Hook>, Tail>
where
    Tail: FoldMountRoutes<Http, TailProof>,
    Hook: RouteRegistrar<Tail::Output, Proof>,
{
    type Output = <Hook as RouteRegistrar<Tail::Output, Proof>>::Output;

    fn fold_mount_routes(self, http: Http) -> Self::Output {
        let http = self.tail.fold_mount_routes(http);
        self.head.value.register_routes(http)
    }
}

/// Fold a routes HList into an axum [`Router`].
pub trait MountRoutes {
    fn mount_routes(self, router: Router<()>) -> Router<()>;
}

impl MountRoutes for HNil {
    fn mount_routes(self, router: Router<()>) -> Router<()> {
        router
    }
}

impl<Tag, Tail> MountRoutes for HCons<Tagged<Tag, Route>, Tail>
where
    Tail: MountRoutes,
{
    fn mount_routes(self, router: Router<()>) -> Router<()> {
        let route = self.head.value;
        let router = router.route(&route.path, route.method_router);
        self.tail.mount_routes(router)
    }
}

/// Publish each mounted capability value into request [`Extensions`].
pub trait ProvideRequestCaps {
    fn provide_request_caps(&self, extensions: &mut Extensions);
}

impl ProvideRequestCaps for HNil {
    fn provide_request_caps(&self, _: &mut Extensions) {}
}

impl<Tag, V, Tail> ProvideRequestCaps for HCons<Tagged<Tag, V>, Tail>
where
    V: Clone + Send + Sync + 'static,
    Tail: ProvideRequestCaps,
{
    fn provide_request_caps(&self, extensions: &mut Extensions) {
        extensions.insert(self.head.value.clone());
        self.tail.provide_request_caps(extensions);
    }
}

/// Axum extractor for a capability value published from the mounted App HList.
///
/// Each request receives clones of mounted capability values via middleware in
/// [`into_axum_router`]. Missing values yield `500`.
///
/// # Use cases
///
/// - Inject database pools, config, or plugin state into handlers without global state.
///
/// # Examples
///
/// ```rust ignore
/// async fn handler(Cap(pool): Cap<Arc<DbPool>>) -> impl IntoResponse {
///     // use pool...
/// }
/// ```
pub struct Cap<T>(pub T);

impl<S, T> axum::extract::FromRequestParts<S> for Cap<T>
where
    T: Clone + Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = (axum::http::StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<T>()
            .cloned()
            .map(Cap)
            .ok_or((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "missing capability in request extensions",
            ))
    }
}

/// Mounted HTTP capability holding a compile-time HList of tagged [`Route`] entries.
///
/// Builder plugins prepend routes; at mount time the list is wrapped in [`Arc`] so
/// per-request clones stay cheap.
#[derive(Clone)]
pub struct HttpCapability<Routes> {
    pub routes: Routes,
}

impl HttpCapability<HNil> {
    pub fn new() -> Self {
        Self { routes: HNil }
    }
}

impl Default for HttpCapability<HNil> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Routes> HttpCapability<Routes> {
    pub fn prepend<Tag>(
        self,
        route: Route,
    ) -> HttpCapability<HCons<Tagged<Tag, Route>, Routes>>
    where
        Routes: HList,
    {
        HttpCapability {
            routes: HCons {
                head: Tagged::new(route),
                tail: self.routes,
            },
        }
    }

    pub fn get_route<Tag, Index>(&self) -> &Route
    where
        Routes: GetByTag<Tag, Index, Value = Route>,
    {
        self.routes.get_by_tag()
    }

    pub fn into_router(self) -> Router<()>
    where
        Routes: MountRoutes,
    {
        self.routes.mount_routes(Router::new())
    }
}

/// Builder-phase HTTP capability (`items` holds [`crate::http::HttpCapability`], `hooks` queues
/// deferred [`RouteRegistrar`] plugins).
pub type HttpCap<Hooks, Http> = CapStore<HttpTag, Hooks, Http>;

impl<Hooks, Routes> HttpCap<Hooks, HttpCapability<Routes>> {
    /// Apply route hooks, then clear the hook list.
    pub fn resolve_route_hooks<Proof>(
        self,
    ) -> HttpCap<HNil, <Hooks as FoldMountRoutes<HttpCapability<Routes>, Proof>>::Output>
    where
        Hooks: FoldMountRoutes<HttpCapability<Routes>, Proof>,
    {
        let http = self.hooks.fold_mount_routes(self.items);
        CapStore::with_items(http)
    }
}

impl<Http> Capability for HttpCap<HNil, Http> {
    /// Shared so request-extension / middleware clones do not deep-copy the route HList
    /// (recursive [`Clone`] of dozens of [`Route`]s overflows the default tokio stack).
    type Value = Arc<Http>;
    type Output = Tagged<HttpTag, Arc<Http>>;
    type Hooks = HNil;
    type Items = Http;

    fn mount(self) -> Self::Output {
        Tagged::new(Arc::new(self.items))
    }
}

/// Add an empty HTTP capability to `app` (call before plugins register routes).
///
/// # Examples
///
/// ```rust
/// # use lariv_rs::{app::App, http::with_http};
/// let app = with_http(App::new());
/// ```
pub fn with_http<L, Proof>(app: App<L>) -> App<HCons<HttpCap<HNil, HttpCapability<HNil>>, L>>
where
    L: HList + CapTagAbsent<HttpTag, Proof>,
{
    app.add_capability(CapStore::with_items(HttpCapability::new()))
}

/// Build the axum [`Router`] from a mounted app: fold routes, inject capability extensions,
/// and apply HTMX middleware (redirect rewrite + `Vary`).
///
/// # Use cases
///
/// - Final step in `main` after [`App::mount`](crate::app::App::mount).
/// - Serve the Lariv app with per-request access to all mounted capabilities.
pub fn into_axum_router<M, HttpIdx, Routes, SlotIdx>(
    app: &MountedApp<M>,
) -> Router
where
    M: GetByTag<HttpTag, HttpIdx, Value = Arc<HttpCapability<Routes>>>,
    M: GetByTag<SlotTag, SlotIdx, Value = SharedChromeFolder>,
    M: ProvideRequestCaps + Clone + Send + Sync + 'static,
    Routes: MountRoutes + Clone,
{
    // One deep clone of routes to hand ownership to axum; mounted value stays behind Arc.
    let router = app
        .get_capability_output::<HttpTag, HttpIdx>()
        .as_ref()
        .clone()
        .into_router();
    // Arc so each request only bumps a refcount instead of recursively cloning the cap HList.
    let caps = Arc::new(app.capabilities.clone());
    router
        .layer(middleware::from_fn(crate::web::htmx_middleware))
        .layer(middleware::from_fn(move |mut req: Request<Body>, next: Next| {
            let caps = Arc::clone(&caps);
            async move {
                caps.provide_request_caps(req.extensions_mut());
                next.run(req).await
            }
        }))
}
