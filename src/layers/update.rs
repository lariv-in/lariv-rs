//! Update layer — POST update; expects model at Acc head under `Key` (from [`DetailLayer`](crate::layers::DetailLayer)).

use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;

use axum::http::Method;
use axum::response::{IntoResponse, Redirect};
use frunk::{HCons, HNil, hlist::HList};

use crate::layers::{
    LayerContrib, LayerRequest, LayerStep, ViewLayer, cons_tagged,
    create::{FormErrorsTag, FormValuesTag},
    patch::FoldFormPatchers,
};
use crate::tag::Tagged;

/// Update an existing model from form values.
pub trait UpdateEntity: Send + Sync {
    type Model: Clone + Send + Sync + 'static;
    type State: Sync;

    fn update_from_form(
        state: &Self::State,
        model: Self::Model,
        values: &HashMap<String, String>,
    ) -> impl Future<Output = Result<Self::Model, String>> + Send;

    fn success_url(model: &Self::Model) -> String;
}

pub trait HasUpdateState<U: UpdateEntity> {
    fn update_state(&self) -> &U::State;
}

pub trait HasFormMapsRef {
    fn form_values(&self) -> &HashMap<String, String>;
}

/// On POST: update entity at Acc head under `Key`; success → redirect; failure → form maps in Data.
pub struct UpdateLayer<Updater, Key, Patchers = HNil>
where
    Updater: UpdateEntity,
{
    pub patchers: Patchers,
    _updater: PhantomData<fn() -> Updater>,
    _key: PhantomData<fn() -> Key>,
}

impl<Updater, Key> UpdateLayer<Updater, Key, HNil>
where
    Updater: UpdateEntity,
{
    pub const fn new() -> Self {
        Self {
            patchers: HNil,
            _updater: PhantomData,
            _key: PhantomData,
        }
    }
}

impl<Updater, Key> Default for UpdateLayer<Updater, Key, HNil>
where
    Updater: UpdateEntity,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Updater, Key, Patchers> UpdateLayer<Updater, Key, Patchers>
where
    Updater: UpdateEntity,
{
    pub fn with_patchers<P>(self, patchers: P) -> UpdateLayer<Updater, Key, P> {
        UpdateLayer {
            patchers,
            _updater: PhantomData,
            _key: PhantomData,
        }
    }
}

impl<Updater, Key, Patchers> LayerContrib for UpdateLayer<Updater, Key, Patchers>
where
    Updater: UpdateEntity,
{
    type Contrib = HCons<
        Tagged<FormValuesTag, HashMap<String, String>>,
        HCons<Tagged<FormErrorsTag, HashMap<String, String>>, HNil>,
    >;
}

impl<Ctx, Tail, Updater, Key, Patchers> ViewLayer<Ctx, HCons<Tagged<Key, Updater::Model>, Tail>>
    for UpdateLayer<Updater, Key, Patchers>
where
    Tail: HList + Send,
    Ctx: HasUpdateState<Updater> + HasFormMapsRef + Send,
    Updater: UpdateEntity,
    Key: Send + Sync + 'static,
    Patchers: FoldFormPatchers + Sync,
{
    type AccOut = HCons<
        Tagged<FormValuesTag, HashMap<String, String>>,
        HCons<
            Tagged<FormErrorsTag, HashMap<String, String>>,
            HCons<Tagged<Key, Updater::Model>, Tail>,
        >,
    >;

    fn run<'a>(
        &'a self,
        ctx: &'a mut Ctx,
        req: &'a mut LayerRequest,
        acc: HCons<Tagged<Key, Updater::Model>, Tail>,
    ) -> impl Future<Output = LayerStep<Self::AccOut>> + Send + 'a
    where
        HCons<Tagged<Key, Updater::Model>, Tail>: Send + 'a,
    {
        async move {
            if req.method != Method::POST {
                return LayerStep::Continue(with_form_maps(
                    HashMap::new(),
                    HashMap::new(),
                    acc,
                ));
            }
            let model = acc.head.value.clone();
            let mut values = ctx.form_values().clone();
            let mut errors = HashMap::new();
            self.patchers.apply_form(&mut values, &mut errors);
            if !errors.is_empty() {
                return LayerStep::Continue(with_form_maps(values, errors, acc));
            }
            match Updater::update_from_form(ctx.update_state(), model, &values).await {
                Ok(updated) => {
                    let url = Updater::success_url(&updated);
                    LayerStep::Done(Redirect::to(&url).into_response())
                }
                Err(e) => {
                    errors.insert("_form".into(), e);
                    LayerStep::Continue(with_form_maps(values, errors, acc))
                }
            }
        }
    }
}

fn with_form_maps<Acc>(
    values: HashMap<String, String>,
    errors: HashMap<String, String>,
    acc: Acc,
) -> HCons<
    Tagged<FormValuesTag, HashMap<String, String>>,
    HCons<Tagged<FormErrorsTag, HashMap<String, String>>, Acc>,
> {
    cons_tagged::<FormValuesTag, _, _>(
        values,
        cons_tagged::<FormErrorsTag, _, _>(errors, acc),
    )
}
