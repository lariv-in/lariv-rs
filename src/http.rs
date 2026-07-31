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
    components::slots::{FoldSlots, SlotTag},
    tag::Tagged,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::GetByTag,
    },
};

/// Capability tag for the HTTP router.
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

/// A route entry in the HTTP capability's typed route HList.
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

    pub fn get<H, T>(path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        Self::new(path, Method::Get, get(handler))
    }

    pub fn post<H, T>(path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        Self::new(path, Method::Post, post(handler))
    }

    pub fn put<H, T>(path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        Self::new(path, Method::Put, put(handler))
    }

    pub fn delete<H, T>(path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        Self::new(path, Method::Delete, delete(handler))
    }

    pub fn patch<H, T>(path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        Self::new(path, Method::Patch, patch(handler))
    }

    pub fn head<H, T>(path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        Self::new(path, Method::Head, head(handler))
    }

    pub fn options<H, T>(path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        Self::new(path, Method::Options, options(handler))
    }

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

/// Plugin hook for appending routes onto an [`HttpCapability`].
pub trait RouteRegistrar<Http, Templates, Slots, Proof = ()>: Sized {
    type Output;
    fn register_routes(self, http: Http) -> Self::Output;
}

/// Apply queued route hooks (tail first so install order is preserved).
pub trait FoldMountRoutes<Templates, Slots, Http, Proof = ()>: Sized {
    type Output;
    fn fold_mount_routes(self, http: Http) -> Self::Output;
}

impl<Templates, Slots, Http> FoldMountRoutes<Templates, Slots, Http> for HNil {
    type Output = Http;

    fn fold_mount_routes(self, http: Http) -> Self::Output {
        http
    }
}

impl<Plugin, Hook, Tail, Templates, Slots, Http, TailProof, Proof>
    FoldMountRoutes<Templates, Slots, Http, (TailProof, Proof)>
    for HCons<Tagged<Plugin, Hook>, Tail>
where
    Tail: FoldMountRoutes<Templates, Slots, Http, TailProof>,
    Hook: RouteRegistrar<Tail::Output, Templates, Slots, Proof>,
{
    type Output = <Hook as RouteRegistrar<Tail::Output, Templates, Slots, Proof>>::Output;

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

/// Extract a capability value published from the mounted App HList into request extensions.
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

/// Mounted HTTP capability: a compile-time HList of tagged [`Route`] entries.
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

/// Builder-phase HTTP capability (`items` holds [`HttpCapability`]).
pub type HttpCap<Hooks, Http> = CapStore<HttpTag, Hooks, Http>;

impl<Hooks, Routes> HttpCap<Hooks, HttpCapability<Routes>> {
    /// Apply [`MountRoutesHook`]s using final template/slot item types, then clear hooks.
    pub fn resolve_route_hooks<Templates, Slots, Proof>(
        self,
    ) -> HttpCap<
        HNil,
        <Hooks as FoldMountRoutes<Templates, Slots, HttpCapability<Routes>, Proof>>::Output,
    >
    where
        Hooks: FoldMountRoutes<Templates, Slots, HttpCapability<Routes>, Proof>,
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

pub fn with_http<L, Proof>(app: App<L>) -> App<HCons<HttpCap<HNil, HttpCapability<HNil>>, L>>
where
    L: HList + CapTagAbsent<HttpTag, Proof>,
{
    app.add_capability(CapStore::with_items(HttpCapability::new()))
}

/// Build the axum router from mounted [`HttpTag`], publishing capability values into extensions.
pub fn into_axum_router<M, HttpIdx, Routes, SlotIdx, Slots>(
    app: &MountedApp<M>,
) -> Router
where
    M: GetByTag<HttpTag, HttpIdx, Value = Arc<HttpCapability<Routes>>>,
    M: GetByTag<SlotTag, SlotIdx, Value = crate::components::SlotCapability<Slots>>,
    M: ProvideRequestCaps + Clone + Send + Sync + 'static,
    Routes: MountRoutes + Clone,
    Slots: FoldSlots + Clone + Send + Sync + 'static,
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
