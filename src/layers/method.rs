//! Method-gated short-circuit layer (Go `MethodLayer`).

use std::future::Future;

use axum::http::Method;
use axum::response::Response;
use frunk::{HNil, hlist::HList};

use crate::layers::{LayerContrib, LayerRequest, LayerStep, ViewLayer};

/// If `req.method` matches, run `handler` and stop the stack; otherwise continue.
pub struct MethodLayer<F> {
    pub method: Method,
    pub handler: F,
}

impl<F> MethodLayer<F> {
    pub fn new(method: Method, handler: F) -> Self {
        Self { method, handler }
    }

    pub fn post(handler: F) -> Self {
        Self::new(Method::POST, handler)
    }

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
