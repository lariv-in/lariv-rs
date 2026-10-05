//! Request auth hooks registered by the users plugin.
//!
//! Core routing and `authorized_role` call these hooks. With the users plugin
//! not installed, scope is a no-op and role checks deny.

use std::any::TypeId;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::OnceLock;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use tower::Service;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

pub type AuthScopeFn = fn(Request, Next) -> BoxFuture<Response>;
pub type AuthorizeFn = fn(Request, Vec<String>, TypeId) -> BoxFuture<Result<Request, Response>>;
pub type RolePermittedFn = fn(&[String]) -> bool;

static AUTH_SCOPE: OnceLock<AuthScopeFn> = OnceLock::new();
static AUTHORIZE: OnceLock<AuthorizeFn> = OnceLock::new();
static ROLE_PERMITTED: OnceLock<RolePermittedFn> = OnceLock::new();

pub fn register_auth_scope(f: AuthScopeFn) {
    let _ = AUTH_SCOPE.set(f);
}

pub fn register_authorize(f: AuthorizeFn) {
    let _ = AUTHORIZE.set(f);
}

pub fn register_role_permitted(f: RolePermittedFn) {
    let _ = ROLE_PERMITTED.set(f);
}

/// Scope the request for handlers. No-op until the users plugin registers.
pub async fn continue_with_auth_scope(req: Request, next: Next) -> Response {
    if let Some(f) = AUTH_SCOPE.get() {
        f(req, next).await
    } else {
        next.run(req).await
    }
}

/// Whether the current principal may see `roles`. False until the users plugin registers.
pub fn role_permitted(roles: &[String]) -> bool {
    ROLE_PERMITTED.get().is_some_and(|f| f(roles))
}

pub async fn authorize_request(
    req: Request,
    roles: Vec<String>,
    tag: TypeId,
) -> Result<Request, Response> {
    if let Some(f) = AUTHORIZE.get() {
        f(req, roles, tag).await
    } else {
        Ok(req)
    }
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
