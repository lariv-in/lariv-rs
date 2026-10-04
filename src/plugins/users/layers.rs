//! View layers for authentication.
//!
//! Role allowlists are enforced by [`super::role_authorization::RoleAuthorizationLayer`].

use std::future::Future;

use axum::response::{IntoResponse, Redirect};
use frunk::{HCons, HNil, hlist::HList};

use crate::layers::{LayerContrib, LayerRequest, LayerStep, ViewLayer, cons_tagged};
use crate::plugins::users::middleware::resolve_auth_headers;
use crate::plugins::users::routes::UsersLoginGetRouteTag;
use crate::plugins::users::state::{AuthContext, UsersState};
use crate::tag::Tagged;

/// Tag for authenticated principal in layer Data.
pub struct AuthTag;

/// Context that exposes [`UsersState`] for auth layers.
pub trait HasUsersState {
    fn users_state(&self) -> &UsersState;
}

/// Mutable slot for the authenticated principal (set by [`AuthLayer`]).
pub trait AuthSlot {
    fn set_auth(&mut self, auth: AuthContext);
    fn auth(&self) -> Option<&AuthContext>;
}

/// Requires a valid session; contributes [`AuthContext`] under [`AuthTag`].
#[derive(Clone, Copy, Debug, Default)]
pub struct AuthLayer;

impl LayerContrib for AuthLayer {
    type Contrib = HCons<Tagged<AuthTag, AuthContext>, HNil>;
}

impl<Ctx, Acc> ViewLayer<Ctx, Acc> for AuthLayer
where
    Acc: HList + Send,
    Ctx: HasUsersState + AuthSlot + Send,
{
    type AccOut = HCons<Tagged<AuthTag, AuthContext>, Acc>;

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
            match resolve_auth_headers(&req.headers, ctx.users_state()).await {
                Some(auth) => {
                    ctx.set_auth(auth.clone());
                    req.auth_present = true;
                    LayerStep::Continue(cons_tagged::<AuthTag, _, _>(auth, acc))
                }
                None => LayerStep::Done(Redirect::to(&UsersLoginGetRouteTag.url()).into_response()),
            }
        }
    }
}

/// Optional auth; contributes `Option<AuthContext>` under [`AuthTag`].
#[derive(Clone, Copy, Debug, Default)]
pub struct OptionalAuthLayer;

impl LayerContrib for OptionalAuthLayer {
    type Contrib = HCons<Tagged<AuthTag, Option<AuthContext>>, HNil>;
}

impl<Ctx, Acc> ViewLayer<Ctx, Acc> for OptionalAuthLayer
where
    Acc: HList + Send,
    Ctx: HasUsersState + AuthSlot + Send,
{
    type AccOut = HCons<Tagged<AuthTag, Option<AuthContext>>, Acc>;

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
            let auth = resolve_auth_headers(&req.headers, ctx.users_state()).await;
            if let Some(ref a) = auth {
                ctx.set_auth(a.clone());
                req.auth_present = true;
            }
            LayerStep::Continue(cons_tagged::<AuthTag, _, _>(auth, acc))
        }
    }
}

impl crate::components::SlotCtx {
    pub fn from_auth(auth: &AuthContext) -> Self {
        Self {
            name: Some(auth.user.name.clone()),
            role: Some(auth.role.clone()),
        }
    }
}
