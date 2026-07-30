//! Delete layer — POST delete; expects model at Acc head under `Key`.

use std::future::Future;
use std::marker::PhantomData;

use axum::http::Method;
use axum::response::{IntoResponse, Redirect};
use frunk::{HCons, HNil, hlist::HList};

use crate::layers::{LayerContrib, LayerRequest, LayerStep, ViewLayer};
use crate::tag::Tagged;

/// Delete an existing model.
pub trait DeleteEntity: Send + Sync {
    type Model: Clone + Send + Sync + 'static;
    type State: Sync;

    fn delete_model(
        state: &Self::State,
        model: Self::Model,
    ) -> impl Future<Output = Result<(), String>> + Send;

    fn success_url() -> &'static str;
}

pub trait HasDeleteState<D: DeleteEntity> {
    fn delete_state(&self) -> &D::State;
}

/// On POST: delete entity at Acc head under `Key` and redirect; on GET: continue to render.
pub struct DeleteLayer<Deleter, Key>
where
    Deleter: DeleteEntity,
{
    _deleter: PhantomData<fn() -> Deleter>,
    _key: PhantomData<fn() -> Key>,
}

impl<Deleter, Key> DeleteLayer<Deleter, Key>
where
    Deleter: DeleteEntity,
{
    pub const fn new() -> Self {
        Self {
            _deleter: PhantomData,
            _key: PhantomData,
        }
    }
}

impl<Deleter, Key> Default for DeleteLayer<Deleter, Key>
where
    Deleter: DeleteEntity,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Deleter, Key> Clone for DeleteLayer<Deleter, Key>
where
    Deleter: DeleteEntity,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Deleter, Key> Copy for DeleteLayer<Deleter, Key> where Deleter: DeleteEntity {}

impl<Deleter, Key> LayerContrib for DeleteLayer<Deleter, Key>
where
    Deleter: DeleteEntity,
{
    type Contrib = HNil;
}

impl<Ctx, Tail, Deleter, Key> ViewLayer<Ctx, HCons<Tagged<Key, Deleter::Model>, Tail>>
    for DeleteLayer<Deleter, Key>
where
    Tail: HList + Send,
    Ctx: HasDeleteState<Deleter> + Send,
    Deleter: DeleteEntity,
    Key: Send + Sync + 'static,
{
    type AccOut = HCons<Tagged<Key, Deleter::Model>, Tail>;

    fn run<'a>(
        &'a self,
        ctx: &'a mut Ctx,
        req: &'a mut LayerRequest,
        acc: HCons<Tagged<Key, Deleter::Model>, Tail>,
    ) -> impl Future<Output = LayerStep<Self::AccOut>> + Send + 'a
    where
        HCons<Tagged<Key, Deleter::Model>, Tail>: Send + 'a,
    {
        async move {
            if req.method != Method::POST {
                return LayerStep::Continue(acc);
            }
            let model = acc.head.value.clone();
            match Deleter::delete_model(ctx.delete_state(), model).await {
                Ok(()) | Err(_) => {
                    LayerStep::Done(Redirect::to(Deleter::success_url()).into_response())
                }
            }
        }
    }
}
