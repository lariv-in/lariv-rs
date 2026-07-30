//! Runtime string→handler view registry (Go `App.Views` / `NewDynamicView`).

use std::collections::HashMap;

use axum::{
    extract::Request,
    handler::Handler,
    response::{IntoResponse, Response},
    routing::{MethodRouter, get},
};
use frunk::{HCons, HNil, hlist::HList};
use http::StatusCode;
use tower::ServiceExt;

use crate::{
    app::App,
    capability::{CapStore, Capability},
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};

/// Capability tag for the named view registry.
pub struct ViewTag;

/// Named HTTP handlers resolved at request time (e.g. PWA `offlineViewName`).
#[derive(Clone, Default)]
pub struct ViewRegistry {
    views: HashMap<String, MethodRouter<()>>,
}

impl ViewRegistry {
    pub fn new() -> Self {
        Self {
            views: HashMap::new(),
        }
    }

    /// Register a GET handler under `name` (overwrites an existing entry).
    pub fn register<H, T>(mut self, name: impl Into<String>, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.views.insert(name.into(), get(handler));
        self
    }

    /// Register an arbitrary [`MethodRouter`] under `name`.
    pub fn register_router(mut self, name: impl Into<String>, router: MethodRouter<()>) -> Self {
        self.views.insert(name.into(), router);
        self
    }

    pub fn get(&self, name: &str) -> Option<&MethodRouter<()>> {
        self.views.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.views.contains_key(name)
    }

    /// Dispatch `req` to the named view, or [`StatusCode::NOT_FOUND`].
    pub async fn dispatch(&self, name: &str, req: Request) -> Response {
        let Some(router) = self.views.get(name) else {
            return StatusCode::NOT_FOUND.into_response();
        };
        match router.clone().oneshot(req).await {
            Ok(res) => res.into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

/// Builder-phase views capability.
pub type ViewCap = CapStore<ViewTag, HNil, ViewRegistry>;

impl Capability for ViewCap {
    type Value = ViewRegistry;
    type Output = Tagged<ViewTag, ViewRegistry>;
    type Hooks = HNil;
    type Items = ViewRegistry;

    fn mount(self) -> Self::Output {
        Tagged::new(self.items)
    }
}

pub fn with_views<L, Proof>(app: App<L>) -> App<HCons<ViewCap, L>>
where
    L: HList + CapTagAbsent<ViewTag, Proof>,
{
    app.add_capability(CapStore::with_items(ViewRegistry::new()))
}
