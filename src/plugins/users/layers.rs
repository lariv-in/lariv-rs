//! View layers for authentication and role authorization (Go `p_users` layers).
//!
//! Use on a typed view stack instead of axum extractors so allowed roles are
//! configured per route via [`RoleLayer::allow`].

use std::future::Future;

use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use frunk::{HCons, HNil, hlist::HList};

use crate::layers::{LayerContrib, LayerRequest, LayerStep, ViewLayer, cons_tagged};
use crate::plugins::users::middleware::{resolve_auth_headers, roles_allowed};
use crate::plugins::users::state::{AuthContext, UsersState};
use crate::tag::Tagged;

/// Tag for authenticated principal in layer Data.
pub struct AuthTag;

/// Context that exposes [`UsersState`] for auth layers.
pub trait HasUsersState {
    fn users_state(&self) -> &UsersState;
}

/// Mutable slot for the authenticated principal (set by [`AuthLayer`], read by [`RoleLayer`]).
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
                None => LayerStep::Done(Redirect::to("/users/login").into_response()),
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

/// Restrict access to an allowlist of role names (superuser always allowed).
///
/// Empty allowlist ⇒ superuser only (same as [`roles_allowed`] with `&[]`).
/// Change roles per route by swapping the slice passed to [`RoleLayer::allow`].
///
/// Expects [`AuthLayer`] to have set [`AuthSlot`] on the run context.
#[derive(Clone, Copy, Debug)]
pub struct RoleLayer {
    pub roles: &'static [&'static str],
}

impl RoleLayer {
    pub const fn allow(roles: &'static [&'static str]) -> Self {
        Self { roles }
    }
}

impl LayerContrib for RoleLayer {
    type Contrib = HNil;
}

impl<Ctx, Acc> ViewLayer<Ctx, Acc> for RoleLayer
where
    Acc: HList + Send,
    Ctx: AuthSlot + Send,
{
    type AccOut = Acc;

    fn run<'a>(
        &'a self,
        ctx: &'a mut Ctx,
        _req: &'a mut LayerRequest,
        acc: Acc,
    ) -> impl Future<Output = LayerStep<Self::AccOut>> + Send + 'a
    where
        Acc: Send + 'a,
    {
        async move {
            let Some(auth) = ctx.auth() else {
                return LayerStep::Done(StatusCode::UNAUTHORIZED.into_response());
            };
            if roles_allowed(auth, self.roles) {
                LayerStep::Continue(acc)
            } else {
                LayerStep::Done(StatusCode::UNAUTHORIZED.into_response())
            }
        }
    }
}

/// Requires `is_superuser` on the authenticated user.
#[derive(Clone, Copy, Debug, Default)]
pub struct SuperuserLayer;

impl LayerContrib for SuperuserLayer {
    type Contrib = HNil;
}

impl<Ctx, Acc> ViewLayer<Ctx, Acc> for SuperuserLayer
where
    Acc: HList + Send,
    Ctx: AuthSlot + Send,
{
    type AccOut = Acc;

    fn run<'a>(
        &'a self,
        ctx: &'a mut Ctx,
        _req: &'a mut LayerRequest,
        acc: Acc,
    ) -> impl Future<Output = LayerStep<Self::AccOut>> + Send + 'a
    where
        Acc: Send + 'a,
    {
        async move {
            let Some(auth) = ctx.auth() else {
                return LayerStep::Done(StatusCode::UNAUTHORIZED.into_response());
            };
            if auth.user.is_superuser {
                LayerStep::Continue(acc)
            } else {
                LayerStep::Done(StatusCode::UNAUTHORIZED.into_response())
            }
        }
    }
}

/// Build [`crate::components::SlotCtx`] from auth.
pub fn slot_ctx_from_auth(auth: &AuthContext) -> crate::components::SlotCtx {
    crate::components::SlotCtx {
        name: Some(auth.user.name.clone()),
        role: Some(auth.role.clone()),
        is_superuser: auth.user.is_superuser,
    }
}

/// Helper used by tests/docs — resolve from headers only.
pub async fn try_auth(headers: &HeaderMap, state: &UsersState) -> Option<AuthContext> {
    resolve_auth_headers(headers, state).await
}

/// Example: two views that differ only by [`RoleLayer`] allowlist.
///
/// ```ignore
/// use lariv_rs::layers::view;
/// use lariv_rs::plugins::users::layers::{AuthLayer, RoleLayer};
///
/// let editors = view::<MyPageTag>()
///     .layer(AuthLayer)
///     .layer(RoleLayer::allow(&["editor", "admin"]));
///
/// let admins_only = view::<MyPageTag>()
///     .layer(AuthLayer)
///     .layer(RoleLayer::allow(&["admin"]));
/// ```
fn _role_layer_doc() {}

#[allow(dead_code)]
fn _response_ty(_: Response) {}
