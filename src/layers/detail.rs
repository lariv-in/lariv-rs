//! Detail load layer — loads one row by path id into Data.

use std::future::Future;
use std::marker::PhantomData;

use axum::response::{IntoResponse, Redirect};
use frunk::{HCons, HNil, hlist::HList};

use crate::layers::{LayerContrib, LayerRequest, LayerStep, ViewLayer, cons_tagged};
use crate::tag::Tagged;

/// Load a model by primary key from plugin state.
pub trait LoadById: Send + Sync {
    type Model: Clone + Send + Sync + 'static;
    type State: Sync;

    fn load_by_id(
        state: &Self::State,
        id: i64,
    ) -> impl Future<Output = Option<Self::Model>> + Send;
}

/// Context that exposes the state used by [`LoadById`].
pub trait HasLoadState<L: LoadById> {
    fn load_state(&self) -> &L::State;
}

/// Loads a record and stores it under `Key` in Data (as HList head).
///
/// Path id is read from [`LayerRequest`] using `path_param` (default `"id"`).
/// Place this immediately before Update/Delete so the model is at the Acc head.
pub struct DetailLayer<Loader, Key>
where
    Loader: LoadById,
{
    pub path_param: &'static str,
    pub missing_redirect: &'static str,
    _loader: PhantomData<fn() -> Loader>,
    _key: PhantomData<fn() -> Key>,
}

impl<Loader, Key> DetailLayer<Loader, Key>
where
    Loader: LoadById,
{
    pub const fn new() -> Self {
        Self {
            path_param: "id",
            missing_redirect: "/",
            _loader: PhantomData,
            _key: PhantomData,
        }
    }

    pub const fn path_param(self, name: &'static str) -> Self {
        Self {
            path_param: name,
            missing_redirect: self.missing_redirect,
            _loader: PhantomData,
            _key: PhantomData,
        }
    }

    pub const fn missing_redirect(self, path: &'static str) -> Self {
        Self {
            path_param: self.path_param,
            missing_redirect: path,
            _loader: PhantomData,
            _key: PhantomData,
        }
    }
}

impl<Loader, Key> Default for DetailLayer<Loader, Key>
where
    Loader: LoadById,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Loader, Key> Clone for DetailLayer<Loader, Key>
where
    Loader: LoadById,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Loader, Key> Copy for DetailLayer<Loader, Key> where Loader: LoadById {}

impl<Loader, Key> LayerContrib for DetailLayer<Loader, Key>
where
    Loader: LoadById,
{
    type Contrib = HCons<Tagged<Key, Loader::Model>, HNil>;
}

impl<Ctx, Acc, Loader, Key> ViewLayer<Ctx, Acc> for DetailLayer<Loader, Key>
where
    Acc: HList + Send,
    Ctx: HasLoadState<Loader> + Send,
    Loader: LoadById,
    Key: Send + Sync + 'static,
{
    type AccOut = HCons<Tagged<Key, Loader::Model>, Acc>;

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
            let Some(id) = req.path_i64(self.path_param) else {
                return LayerStep::Done(Redirect::to(self.missing_redirect).into_response());
            };
            let Some(model) = Loader::load_by_id(ctx.load_state(), id).await else {
                return LayerStep::Done(Redirect::to(self.missing_redirect).into_response());
            };
            LayerStep::Continue(cons_tagged::<Key, _, _>(model, acc))
        }
    }
}
