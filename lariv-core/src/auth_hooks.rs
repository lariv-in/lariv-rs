//! Auth callbacks carried on the capability stack.
//!
//! The users plugin attaches [`AuthHooks`] at install. Mount publishes the value into
//! request extensions. Routing and `authorized_role` read it from the request. With the
//! capability absent, scope is a no-op, authorization passes the request through, and
//! role checks deny.
//!
//! Sync template code has no request, so the request middleware copies the mounted value
//! into a task-local for the rest of that request. The task-local is not a process-wide
//! registration.

use std::any::TypeId;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{
    capability::{Capability, HasCapTag},
    tag::Tagged,
};
use axum::body::Body;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use tower::Service;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

pub type AuthScopeFn = fn(Request, Next) -> BoxFuture<Response>;
pub type AuthorizeFn = fn(Request, Vec<String>, TypeId) -> BoxFuture<Result<Request, Response>>;
pub type RolePermittedFn = fn(&[String]) -> bool;

tokio::task_local! {
    static CURRENT_AUTH_HOOKS: AuthHooks;
}

/// Capability tag for [`AuthHooks`].
pub struct AuthHooksTag;

/// Callbacks the users plugin installs. Absent fields keep the core defaults.
#[derive(Clone, Copy, Default)]
pub struct AuthHooks {
    scope: Option<AuthScopeFn>,
    authorize: Option<AuthorizeFn>,
    role_permitted: Option<RolePermittedFn>,
}

/// Builder-phase store for [`AuthHooks`]. Generic over unused hooks so `cap_attach` can name it.
#[derive(Clone, Copy)]
pub struct AuthHooksCap<Hooks> {
    items: AuthHooks,
    _hooks: PhantomData<Hooks>,
    _tag: PhantomData<fn() -> AuthHooksTag>,
}

impl<Hooks> AuthHooksCap<Hooks> {
    pub const fn new(items: AuthHooks) -> Self {
        Self {
            items,
            _hooks: PhantomData,
            _tag: PhantomData,
        }
    }
}

impl<Hooks> HasCapTag for AuthHooksCap<Hooks> {
    type Tag = AuthHooksTag;
}

impl<Hooks> Capability for AuthHooksCap<Hooks> {
    type Value = AuthHooks;
    type Output = Tagged<AuthHooksTag, AuthHooks>;
    type Hooks = Hooks;
    type Items = AuthHooks;

    fn mount(self) -> Self::Output {
        Tagged::new(self.items)
    }
}

impl AuthHooks {
    pub const fn new(
        scope: AuthScopeFn,
        authorize: AuthorizeFn,
        role_permitted: RolePermittedFn,
    ) -> Self {
        Self {
            scope: Some(scope),
            authorize: Some(authorize),
            role_permitted: Some(role_permitted),
        }
    }
}

/// Scope the request for handlers. No-op when [`AuthHooks`] was not attached.
pub async fn continue_with_auth_scope(req: Request, next: Next) -> Response {
    let hooks = req
        .extensions()
        .get::<AuthHooks>()
        .copied()
        .unwrap_or_default();
    CURRENT_AUTH_HOOKS
        .scope(hooks, async move {
            if let Some(scope) = hooks.scope {
                scope(req, next).await
            } else {
                next.run(req).await
            }
        })
        .await
}

/// Whether the current principal may see `roles`. False when no hooks are scoped.
pub fn role_permitted(roles: &[String]) -> bool {
    CURRENT_AUTH_HOOKS
        .try_with(|hooks| hooks.role_permitted.is_some_and(|f| f(roles)))
        .unwrap_or(false)
}

pub async fn authorize_request(
    req: Request,
    roles: Vec<String>,
    tag: TypeId,
) -> Result<Request, Response> {
    let authorize = req
        .extensions()
        .get::<AuthHooks>()
        .and_then(|hooks| hooks.authorize);
    if let Some(authorize) = authorize {
        authorize(req, roles, tag).await
    } else {
        Ok(req)
    }
}

/// Run `f` with `hooks` visible to [`role_permitted`]. Tests and non-request callers use this.
pub fn scope_sync<R>(hooks: AuthHooks, f: impl FnOnce() -> R) -> R {
    CURRENT_AUTH_HOOKS.sync_scope(hooks, f)
}

/// Tower layer applied by [`crate::http::Route::authorize`].
#[derive(Debug)]
pub struct AuthorizeLayer<Tag> {
    roles: Vec<String>,
    _tag: PhantomData<fn() -> Tag>,
}

impl<Tag> Clone for AuthorizeLayer<Tag> {
    fn clone(&self) -> Self {
        Self {
            roles: self.roles.clone(),
            _tag: PhantomData,
        }
    }
}

impl<Tag> AuthorizeLayer<Tag> {
    pub fn allow(roles: Vec<String>) -> Self {
        Self {
            roles,
            _tag: PhantomData,
        }
    }
}

impl<Tag, S> tower::Layer<S> for AuthorizeLayer<Tag>
where
    Tag: Send + Sync + 'static,
{
    type Service = AuthorizeService<Tag, S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthorizeService {
            roles: self.roles.clone(),
            inner,
            _tag: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct AuthorizeService<Tag, S> {
    roles: Vec<String>,
    inner: S,
    _tag: PhantomData<fn() -> Tag>,
}

impl<Tag, S: Clone> Clone for AuthorizeService<Tag, S> {
    fn clone(&self) -> Self {
        Self {
            roles: self.roles.clone(),
            inner: self.inner.clone(),
            _tag: PhantomData,
        }
    }
}

impl<Tag, S> Service<Request<Body>> for AuthorizeService<Tag, S>
where
    Tag: Send + Sync + 'static,
    S: Service<Request<Body>, Response = Response, Error = std::convert::Infallible>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = std::convert::Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response, std::convert::Infallible>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), std::convert::Infallible>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let roles = self.roles.clone();
        let tag = TypeId::of::<Tag>();
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(async move {
            match authorize_request(req, roles, tag).await {
                Ok(req) => Ok(inner.call(req).await.unwrap_or_else(|err| match err {})),
                Err(response) => Ok(response),
            }
        })
    }
}
