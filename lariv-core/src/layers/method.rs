//! Method-gated short-circuit layer — rejects requests whose HTTP method does not match.
//!
//! When the request method matches, runs a custom handler and stops the layer stack.
//! Otherwise continues to downstream layers — useful for POST-only side effects on
//! shared routes.
//!
//! # Use cases
//!
//! - Handle POST on a detail route without a separate axum handler.
//! - Fire-and-forget actions (toggle, reorder) that return raw `Response`.
//!
//! # Examples
//!
//! ```rust ignore
//! view::<ItemDetailPage>()
//!     .layer(MethodLayer::post(|ctx, req, acc| async {
//!         do_reorder(ctx, req).await.into_response()
//!     }))
//!     .layer(DetailLayer::<ItemLoader, ItemTag>::new())
//! ```

use std::future::Future;

use axum::http::Method;
use axum::response::Response;
use frunk::{HNil, hlist::HList};

use crate::layers::{LayerContrib, LayerRequest, LayerStep, ViewLayer};

/// If `req.method` matches, run `handler` and stop the stack; otherwise continue.
pub struct MethodLayer<F> {
    /// HTTP method that triggers the handler (e.g. [`Method::POST`]).
    pub method: Method,
    /// Called with plugin context, request, and current accumulator when method matches.
    pub handler: F,
}

impl<F> MethodLayer<F> {
    /// Gate on an arbitrary HTTP method.
    pub fn new(method: Method, handler: F) -> Self {
        Self { method, handler }
    }

    /// Shorthand for POST-gated handler.
    pub fn post(handler: F) -> Self {
        Self::new(Method::POST, handler)
    }

    /// Shorthand for GET-gated handler.
    pub fn get(handler: F) -> Self {
        Self::new(Method::GET, handler)
    }
}

impl<F> LayerContrib for MethodLayer<F> {
    type Contrib = HNil;
}

impl<Ctx, Acc, F, Fut> ViewLayer<Ctx, Acc> for MethodLayer<F>
where
    Acc: HList + Send,
    Ctx: Send,
    F: Fn(&mut Ctx, &LayerRequest, &Acc) -> Fut + Sync,
    Fut: Future<Output = Response> + Send,
{
    type AccOut = Acc;

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
            if req.method == self.method {
                LayerStep::Done((self.handler)(ctx, req, &acc).await)
            } else {
                LayerStep::Continue(acc)
            }
        }
    }
}
