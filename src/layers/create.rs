//! Create layer — POST handler that inserts a new record from form values.
//!
//! On GET, contributes empty form maps and continues to render. On POST, validates via
//! form patchers, creates the entity, and redirects on success.
//!
//! # Use cases
//!
//! - "New user" / "New proposal" forms with validation error re-render.
//! - Redirect to detail page after successful create (`success_url_prefix` + id).
//!
//! # Examples
//!
//! ```rust ignore
//! view::<ProposalCreatePage>()
//!     .layer(CreateLayer::<ProposalCreator, _>::new("/proposals/"))
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;

use axum::http::Method;
use axum::response::{IntoResponse, Redirect, Response};
use frunk::{HCons, HNil, hlist::HList};

use crate::layers::{
    LayerContrib, LayerRequest, LayerStep, ViewLayer, cons_tagged,
    patch::FoldFormPatchers,
    update::HasFormMapsRef,
};
use crate::tag::Tagged;

/// Tag for the last created entity id.
pub struct CreatedIdTag;

/// Tag for form field values (string map).
pub struct FormValuesTag;

/// Tag for form field errors (string map).
pub struct FormErrorsTag;

/// Create a record from form values held on the run context.
pub trait CreateEntity: Send + Sync {
    type Model: Clone + Send + Sync + 'static;
    type State: Sync;

    fn create_from_form(
        state: &Self::State,
        values: &HashMap<String, String>,
    ) -> impl Future<Output = Result<Self::Model, String>> + Send;

    fn created_id(model: &Self::Model) -> i64;
}

pub trait HasCreateState<C: CreateEntity> {
    fn create_state(&self) -> &C::State;
}

/// Optional form maps supplied by the handler (multipart/urlencoded already parsed).
pub trait HasFormMaps {
    fn form_values(&self) -> &HashMap<String, String>;
}

impl<T: HasFormMapsRef> HasFormMaps for T {
    fn form_values(&self) -> &HashMap<String, String> {
        HasFormMapsRef::form_values(self)
    }
}

/// On POST: create entity from form values; on success redirect to `success_url_prefix` + id.
///
/// On validation or persistence failure, stashes [`FormValuesTag`] and [`FormErrorsTag`] in
/// Data and continues so the page can re-render the form with errors.
pub struct CreateLayer<Creator, Patchers = HNil>
where
    Creator: CreateEntity,
{
    /// Prefix for post-create redirect (entity id is appended).
    pub success_url_prefix: &'static str,
    pub patchers: Patchers,
    _creator: PhantomData<fn() -> Creator>,
}

impl<Creator> CreateLayer<Creator, HNil>
where
    Creator: CreateEntity,
{
    /// Create layer redirecting to `{prefix}{created_id}` on success.
    pub const fn new(success_url_prefix: &'static str) -> Self {
        Self {
            success_url_prefix,
            patchers: HNil,
            _creator: PhantomData,
        }
    }
}

impl<Creator, Patchers> CreateLayer<Creator, Patchers>
where
    Creator: CreateEntity,
{
    /// Attach an HList of [`FormPatcher`](crate::layers::FormPatcher)s run before create.
    pub fn with_patchers<P>(self, patchers: P) -> CreateLayer<Creator, P> {
        CreateLayer {
            success_url_prefix: self.success_url_prefix,
            patchers,
            _creator: PhantomData,
        }
    }
}

impl<Creator, Patchers> LayerContrib for CreateLayer<Creator, Patchers>
where
    Creator: CreateEntity,
{
    type Contrib = HCons<
        Tagged<FormValuesTag, HashMap<String, String>>,
        HCons<Tagged<FormErrorsTag, HashMap<String, String>>, HNil>,
    >;
}

impl<Ctx, Acc, Creator, Patchers> ViewLayer<Ctx, Acc> for CreateLayer<Creator, Patchers>
where
    Acc: HList + Send,
    Ctx: HasCreateState<Creator> + HasFormMaps + Send,
    Creator: CreateEntity,
    Patchers: FoldFormPatchers + Sync,
{
    type AccOut = HCons<
        Tagged<FormValuesTag, HashMap<String, String>>,
        HCons<Tagged<FormErrorsTag, HashMap<String, String>>, Acc>,
    >;

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
            if req.method != Method::POST {
                return LayerStep::Continue(with_form_maps(HashMap::new(), HashMap::new(), acc));
            }
            let mut values = ctx.form_values().clone();
            let mut errors = HashMap::new();
            self.patchers.apply_form(&mut values, &mut errors);
            if !errors.is_empty() {
                return LayerStep::Continue(with_form_maps(values, errors, acc));
            }
            match Creator::create_from_form(ctx.create_state(), &values).await {
                Ok(model) => {
                    let id = Creator::created_id(&model);
                    let url = format!("{}{id}", self.success_url_prefix);
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

/// Build a redirect response helper used by CUD layers.
#[allow(dead_code)]
pub fn redirect_response(url: &str) -> Response {
    Redirect::to(url).into_response()
}
