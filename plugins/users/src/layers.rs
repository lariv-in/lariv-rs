//! View layers for authentication.
//!
//! Role allowlists are enforced by [`super::role_authorization::RoleAuthorizationLayer`].

use std::future::Future;

use axum::response::{IntoResponse, Redirect};
use frunk::{HCons, HNil, hlist::HList};

use crate::middleware::resolve_request_auth;
use crate::routes::UsersLoginGetRouteTag;
use crate::state::{AuthContext, UsersState};
use lariv_core::layers::{LayerContrib, LayerRequest, LayerStep, ViewLayer, cons_tagged};
use lariv_core::tag::Tagged;

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
            match resolve_request_auth(&req.headers, ctx.users_state()).await {
                Some(resolved) => {
                    ctx.set_auth(resolved.effective.clone());
                    req.auth_present = true;
                    LayerStep::Continue(cons_tagged::<AuthTag, _, _>(resolved.effective, acc))
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
            let auth = resolve_request_auth(&req.headers, ctx.users_state())
                .await
                .map(|resolved| resolved.effective);
            if let Some(ref a) = auth {
                ctx.set_auth(a.clone());
                req.auth_present = true;
            }
            LayerStep::Continue(cons_tagged::<AuthTag, _, _>(auth, acc))
        }
    }
}

impl lariv_core::components::AuthPrincipal for AuthContext {
    fn principal_name(&self) -> &str {
        &self.user.name
    }

    fn principal_role(&self) -> &str {
        &self.role
    }
}
