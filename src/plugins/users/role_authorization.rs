//! Patchable role allowlists and the HTTP [`RoleAuthorizationLayer`].
//!
//! Plugins register a `Vec<String>` per permission tag through [`RoleAuthorizationRegistrar`].
//! Later `cap_hook`s call [`RoleAuthorizationRegistry::patch`] to change that vec. At request
//! time the registry entry wins over the fallback stored on the layer. Superuser always passes.
//! An empty allowlist means superuser only. Denial is `401`.

use std::{
    any::TypeId,
    collections::HashMap,
    convert::Infallible,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use frunk::{HCons, HNil, hlist::HList};
use sea_orm::QueryFilter;
use tower::Service;

use crate::{
    app::App,
    capability::{CapHookExt, Capability, HasCapTag},
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};

use super::{
    middleware::resolve_auth_headers,
    routes::UsersLoginGetRouteTag,
    state::{AuthContext, UsersState},
};

tokio::task_local! {
    static CURRENT_AUTH: AuthContext;
    static CURRENT_ROLES: RoleAuthorizationRegistry;
}

/// Capability tag for the role allowlist registry.
pub struct RoleAuthorizationTag;

/// Folded map of permission tag → allowed role names.
#[derive(Clone, Debug, Default)]
pub struct RoleAuthorizationRegistry {
    rules: HashMap<TypeId, Vec<String>>,
}

impl RoleAuthorizationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the allowlist for `Tag`.
    pub fn allow<Tag: 'static>(mut self, roles: Vec<String>) -> Self {
        self.rules.insert(TypeId::of::<Tag>(), roles);
        self
    }

    /// Edit the allowlist for `Tag`, creating an empty one when missing.
    pub fn patch<Tag: 'static>(mut self, f: impl FnOnce(&mut Vec<String>)) -> Self {
        f(self.rules.entry(TypeId::of::<Tag>()).or_default());
        self
    }

    pub fn contains<Tag: 'static>(&self) -> bool {
        self.rules.contains_key(&TypeId::of::<Tag>())
    }

    /// Allowlist for `Tag`, or an empty slice when the tag was never registered.
    pub fn roles<Tag: 'static>(&self) -> &[String] {
        self.rules
            .get(&TypeId::of::<Tag>())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

/// Plugin hook that folds allowlists into [`RoleAuthorizationRegistry`].
pub trait RoleAuthorizationRegistrar: Sized {
    fn register_roles(self, registry: RoleAuthorizationRegistry) -> RoleAuthorizationRegistry;
}

/// Builder-phase role authorization capability.
#[derive(Clone, Default)]
pub struct RoleAuthorizationCap<Hooks> {
    pub hooks: Hooks,
    pub items: RoleAuthorizationRegistry,
    _tag: PhantomData<fn() -> RoleAuthorizationTag>,
}

impl<Hooks> RoleAuthorizationCap<Hooks> {
    pub fn new() -> Self
    where
        Hooks: Default,
    {
        Self {
            hooks: Hooks::default(),
            items: RoleAuthorizationRegistry::new(),
            _tag: PhantomData,
        }
    }

    pub fn add_hook<HTag, H>(self, hook: H) -> RoleAuthorizationCap<HCons<Tagged<HTag, H>, Hooks>> {
        RoleAuthorizationCap {
            hooks: HCons {
                head: Tagged::new(hook),
                tail: self.hooks,
            },
            items: self.items,
            _tag: PhantomData,
        }
    }
}

impl<Hooks> HasCapTag for RoleAuthorizationCap<Hooks> {
    type Tag = RoleAuthorizationTag;
}

impl<Hooks, Plugin, Hook> CapHookExt<Plugin, Hook> for RoleAuthorizationCap<Hooks> {
    type Hooked = RoleAuthorizationCap<HCons<Tagged<Plugin, Hook>, Hooks>>;

    fn prepend_cap_hook(self, hook: Hook) -> Self::Hooked {
        self.add_hook::<Plugin, Hook>(hook)
    }
}

/// Fold registrar hooks over the registry (tail first = install order).
pub trait FoldRoleAuthorizationHooks {
    fn fold(self, registry: RoleAuthorizationRegistry) -> RoleAuthorizationRegistry;
}

impl FoldRoleAuthorizationHooks for HNil {
    fn fold(self, registry: RoleAuthorizationRegistry) -> RoleAuthorizationRegistry {
        registry
    }
}

impl<Plugin, H, Tail> FoldRoleAuthorizationHooks for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: FoldRoleAuthorizationHooks,
    H: RoleAuthorizationRegistrar + Copy,
{
    fn fold(self, registry: RoleAuthorizationRegistry) -> RoleAuthorizationRegistry {
        let registry = self.tail.fold(registry);
        self.head.value.register_roles(registry)
    }
}

impl<Hooks> Capability for RoleAuthorizationCap<Hooks>
where
    Hooks: FoldRoleAuthorizationHooks,
{
    type Value = RoleAuthorizationRegistry;
    type Output = Tagged<RoleAuthorizationTag, RoleAuthorizationRegistry>;
    type Hooks = Hooks;
    type Items = RoleAuthorizationRegistry;

    fn mount(self) -> Self::Output {
        Tagged::new(self.hooks.fold(self.items))
    }
}

/// Attach an empty role-authorization capability (prefer `cap_attach` in install steps).
pub fn with_role_authorization<L, Proof>(app: App<L>) -> App<HCons<RoleAuthorizationCap<HNil>, L>>
where
    L: HList + CapTagAbsent<RoleAuthorizationTag, Proof>,
{
    app.add_capability(RoleAuthorizationCap::<HNil>::new())
}

/// Allowlist registered for `Tag` on this request, or empty when no registry is scoped.
pub fn roles_for<Tag: 'static>() -> Vec<String> {
    CURRENT_ROLES
        .try_with(|registry| registry.roles::<Tag>().to_vec())
        .unwrap_or_default()
}

/// Authenticated principal for this request, when auth middleware scoped one.
pub fn current_auth() -> Option<AuthContext> {
    CURRENT_AUTH.try_with(|auth| auth.clone()).ok()
}

pub fn principal_allowed(auth: &AuthContext, roles: &[String]) -> bool {
    auth.user.is_superuser || roles.iter().any(|role| role == &auth.role)
}

/// Keep `query` when the current principal may use `Tag`. Any other role matches nothing.
pub fn scope_allowed<Tag, E>(query: sea_orm::Select<E>) -> sea_orm::Select<E>
where
    Tag: 'static,
    E: sea_orm::EntityTrait,
{
    let allowed = current_auth().is_some_and(|auth| principal_allowed(&auth, &roles_for::<Tag>()));
    if allowed {
        query
    } else {
        query.filter(sea_orm::sea_query::Expr::cust("1 = 0"))
    }
}

/// Effective allowlist: the registry entry when present, otherwise `fallback`.
pub fn effective_roles<Tag: 'static>(
    registry: &RoleAuthorizationRegistry,
    fallback: &[String],
) -> Vec<String> {
    if registry.contains::<Tag>() {
        registry.roles::<Tag>().to_vec()
    } else {
        fallback.to_vec()
    }
}

/// Scope the request's auth (if any) and role registry for handlers and `authorized_role`.
pub async fn continue_with_auth_scope(mut req: Request, next: Next) -> Response {
    let registry = req
        .extensions()
        .get::<RoleAuthorizationRegistry>()
        .cloned()
        .unwrap_or_default();
    let auth = match req.extensions().get::<UsersState>().cloned() {
        Some(state) => resolve_auth_headers(req.headers(), &state).await,
        None => None,
    };
    if let Some(ref auth) = auth {
        req.extensions_mut().insert(auth.clone());
    }
    match auth {
        Some(auth) => {
            CURRENT_ROLES
                .scope(registry, CURRENT_AUTH.scope(auth, next.run(req)))
                .await
        }
        None => CURRENT_ROLES.scope(registry, next.run(req)).await,
    }
}

/// Run `f` with a principal and registry, for tests and non-request callers.
pub fn with_principal<R>(
    auth: AuthContext,
    registry: RoleAuthorizationRegistry,
    f: impl FnOnce() -> R,
) -> R {
    CURRENT_ROLES.sync_scope(registry, || CURRENT_AUTH.sync_scope(auth, f))
}

/// Tower layer that allows `Tag`'s roles (and any superuser) before the handler runs.
#[derive(Debug)]
pub struct RoleAuthorizationLayer<Tag> {
    roles: Vec<String>,
    _tag: PhantomData<fn() -> Tag>,
}

impl<Tag> Clone for RoleAuthorizationLayer<Tag> {
    fn clone(&self) -> Self {
        Self {
            roles: self.roles.clone(),
            _tag: PhantomData,
        }
    }
}

impl<Tag> RoleAuthorizationLayer<Tag> {
    pub fn allow(roles: Vec<String>) -> Self {
        Self {
            roles,
            _tag: PhantomData,
        }
    }
}

impl<Tag, S> tower::Layer<S> for RoleAuthorizationLayer<Tag>
where
    Tag: 'static,
{
    type Service = RoleAuthorizationService<Tag, S>;

    fn layer(&self, inner: S) -> Self::Service {
        RoleAuthorizationService {
            roles: self.roles.clone(),
            inner,
            _tag: PhantomData,
        }
    }
}

/// Service produced by [`RoleAuthorizationLayer`].
#[derive(Debug)]
pub struct RoleAuthorizationService<Tag, S> {
    roles: Vec<String>,
    inner: S,
    _tag: PhantomData<fn() -> Tag>,
}

impl<Tag, S: Clone> Clone for RoleAuthorizationService<Tag, S> {
    fn clone(&self) -> Self {
        Self {
            roles: self.roles.clone(),
            inner: self.inner.clone(),
            _tag: PhantomData,
        }
    }
}

impl<Tag, S> Service<Request<Body>> for RoleAuthorizationService<Tag, S>
where
    Tag: 'static,
    S: Service<Request<Body>, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let roles = self.roles.clone();
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(async move {
            let users = req
                .extensions()
                .get::<UsersState>()
                .cloned()
                .unwrap_or_else(|| {
                    panic!("UsersState missing from request; is the users plugin installed?")
                });
            let auth = if let Some(auth) = req.extensions().get::<AuthContext>().cloned() {
                auth
            } else if let Some(auth) = resolve_auth_headers(req.headers(), &users).await {
                auth
            } else {
                return Ok(Redirect::to(&UsersLoginGetRouteTag.url()).into_response());
            };
            let registry = req
                .extensions()
                .get::<RoleAuthorizationRegistry>()
                .cloned()
                .unwrap_or_default();
            let allowed = effective_roles::<Tag>(&registry, &roles);
            if principal_allowed(&auth, &allowed) {
                Ok(inner.call(req).await.unwrap_or_else(|err| match err {}))
            } else {
                Ok(StatusCode::UNAUTHORIZED.into_response())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::plugins::users::entities::user::Model as User;

    struct ViewTag;
    struct MutateTag;

    fn auth(is_superuser: bool, role: &str) -> AuthContext {
        AuthContext {
            user: User {
                id: 1,
                created_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
                name: "Ada".into(),
                email: "ada@example.com".into(),
                phone: "1".into(),
                is_superuser,
                role_id: 1,
                password_hash: Some(vec![]),
                password_salt: Some(vec![]),
                timezone: "UTC".into(),
            },
            role: role.into(),
            timezone: "UTC".into(),
        }
    }

    #[test]
    fn allow_then_patch_changes_the_vec() {
        let registry = RoleAuthorizationRegistry::new()
            .allow::<ViewTag>(vec!["employee".into()])
            .patch::<ViewTag>(|roles| roles.push("manager".into()));
        assert_eq!(
            registry.roles::<ViewTag>(),
            &["employee".to_string(), "manager".to_string()]
        );
    }

    #[test]
    fn patch_on_missing_tag_starts_empty() {
        let registry =
            RoleAuthorizationRegistry::new().patch::<MutateTag>(|roles| roles.push("admin".into()));
        assert_eq!(registry.roles::<MutateTag>(), &["admin".to_string()]);
    }

    #[test]
    fn registry_wins_over_fallback() {
        let registry = RoleAuthorizationRegistry::new().allow::<ViewTag>(vec!["manager".into()]);
        let roles = effective_roles::<ViewTag>(&registry, &["employee".into()]);
        assert_eq!(roles, vec!["manager".to_string()]);
    }

    #[test]
    fn missing_tag_uses_fallback() {
        let registry = RoleAuthorizationRegistry::new();
        let roles = effective_roles::<ViewTag>(&registry, &["employee".into()]);
        assert_eq!(roles, vec!["employee".to_string()]);
    }

    #[test]
    fn empty_allowlist_is_superuser_only() {
        let superuser = auth(true, "employee");
        let employee = auth(false, "employee");
        assert!(principal_allowed(&superuser, &[]));
        assert!(!principal_allowed(&employee, &[]));
    }

    #[test]
    fn named_role_is_allowed() {
        let employee = auth(false, "employee");
        assert!(principal_allowed(&employee, &["employee".into()]));
        assert!(!principal_allowed(&employee, &["admin".into()]));
    }

    #[test]
    fn later_hook_patches_earlier_allow() {
        #[derive(Clone, Copy)]
        struct Owner;
        impl RoleAuthorizationRegistrar for Owner {
            fn register_roles(
                self,
                registry: RoleAuthorizationRegistry,
            ) -> RoleAuthorizationRegistry {
                registry.allow::<ViewTag>(vec!["employee".into()])
            }
        }
        #[derive(Clone, Copy)]
        struct Addon;
        impl RoleAuthorizationRegistrar for Addon {
            fn register_roles(
                self,
                registry: RoleAuthorizationRegistry,
            ) -> RoleAuthorizationRegistry {
                registry.patch::<ViewTag>(|roles| roles.push("manager".into()))
            }
        }
        let hooks = HCons {
            head: Tagged::<(), _>::new(Addon),
            tail: HCons {
                head: Tagged::<(), _>::new(Owner),
                tail: HNil,
            },
        };
        let registry = hooks.fold(RoleAuthorizationRegistry::new());
        assert_eq!(
            registry.roles::<ViewTag>(),
            &["employee".to_string(), "manager".to_string()]
        );
    }
}
