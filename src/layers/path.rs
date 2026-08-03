//! Path parameter extraction layer — parses route placeholders into layer Data.
//!
//! Copies named segments from [`LayerRequest::path`] into layer Data under [`PathTag`].
//! Use at the top of stacks that need typed path maps or before detail/list scoping.
//!
//! # Use cases
//!
//! - Expose `{id}` or `{userId}` to page `BuildFromData` via [`PathTag`].
//! - Declare which path params a view expects (compile-time stack documentation).
//!
//! # Examples
//!
//! ```rust ignore
//! view::<UserEditPage>()
//!     .layer(PathLayer::names(&["userId"]))
//!     .layer(DetailLayer::<UserLoader, UserTag>::new().path_param("userId"))
//! ```

use std::future::Future;

use frunk::{HCons, HNil, hlist::HList};

use crate::layers::{LayerContrib, LayerRequest, LayerStep, ViewLayer, cons_tagged};
use crate::tag::Tagged;

/// Tag for the path-parameter map in layer Data.
pub struct PathTag;

pub type PathMap = std::collections::HashMap<String, String>;

/// Extracts named path parameters from [`LayerRequest::path`] into Data.
///
/// When `names` is empty ([`PathLayer::all`]), copies the entire path map.
#[derive(Clone, Copy, Debug)]
pub struct PathLayer {
    /// Path parameter names to copy (e.g. `&["id", "userId"]`).
    pub names: &'static [&'static str],
}

impl PathLayer {
    /// Extract only the listed path parameter names.
    pub const fn names(names: &'static [&'static str]) -> Self {
        Self { names }
    }

    /// Copy all path parameters present on the request.
    pub const fn all() -> Self {
        Self { names: &[] }
    }
}

impl LayerContrib for PathLayer {
    type Contrib = HCons<Tagged<PathTag, PathMap>, HNil>;
}

impl<Ctx, Acc> ViewLayer<Ctx, Acc> for PathLayer
where
    Acc: HList + Send,
    Ctx: Send,
{
    type AccOut = HCons<Tagged<PathTag, PathMap>, Acc>;

    fn run<'a>(
        &'a self,
        _ctx: &'a mut Ctx,
        req: &'a mut LayerRequest,
        acc: Acc,
    ) -> impl Future<Output = LayerStep<Self::AccOut>> + Send + 'a
    where
        Acc: Send + 'a,
    {
        async move {
            let map = if self.names.is_empty() {
                req.path.clone()
            } else {
                let mut m = PathMap::new();
                for name in self.names {
                    if let Some(v) = req.path.get(*name) {
                        m.insert((*name).to_string(), v.clone());
                    }
                }
                m
            };
            LayerStep::Continue(cons_tagged::<PathTag, _, _>(map, acc))
        }
    }
}
