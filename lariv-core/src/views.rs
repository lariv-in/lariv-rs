//! Runtime string→handler view registry for dynamic page dispatch.
//!
//! Named handlers are resolved at request time — useful for PWA offline views, admin
//! diagnostics, or any route whose target is chosen dynamically.
//!
//! # Routes
//!
//! Register GET handlers (or full [`MethodRouter`]s) under string keys via
//! [`ViewRegistry::register`]. Dispatch with [`ViewRegistry::dispatch`].
//!
//! # Use cases
//!
//! - PWA service worker requests a named offline HTML view.
//! - Plugin exposes a handler map without compile-time route tags.
//! - Fallback or A/B routes selected by configuration at runtime.
//!
//! # Examples
//!
//! ```rust ignore
//! let app = with_views(with_http(App::new()))
//!     .map_capability(|cap| cap.register("offline", offline_handler));
//!
//! // At request time:
//! registry.dispatch("offline", req).await
//! ```

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

/// Capability tag for the named view registry on the app HList.
pub struct ViewTag;

/// Named HTTP handlers resolved at request time.
///
/// Backed by a `HashMap<String, MethodRouter>`. Cloned cheaply for dispatch.
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use lariv_rs::views::ViewRegistry;
    /// let registry = ViewRegistry::new().register("health", || async { "ok" });
    /// assert!(registry.contains("health"));
    /// ```
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

    /// Look up a registered router by name.
    pub fn get(&self, name: &str) -> Option<&MethodRouter<()>> {
        self.views.get(name)
    }

    /// Returns `true` if `name` is registered.
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

/// Builder-phase views capability (no deferred hooks; register directly on `items`).
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

/// Add an empty [`ViewRegistry`] to `app`.
///
/// # Examples
///
/// ```rust
/// # use lariv_rs::{app::App, views::with_views};
/// let app = with_views(App::new());
/// ```
pub fn with_views<L, Proof>(app: App<L>) -> App<HCons<ViewCap, L>>
where
    L: HList + CapTagAbsent<ViewTag, Proof>,
{
    app.add_capability(CapStore::with_items(ViewRegistry::new()))
}
