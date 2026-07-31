use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::EntityTrait;

use crate::{
    http::Cap,
    plugins::users::{
        auth,
        entities::user::Entity as UserEntity,
        error::UsersError,
        jwt,
        session::{self, AUTH_COOKIE},
        state::{AuthContext, UsersState},
    },
};

async fn resolve_auth(parts: &Parts, state: &UsersState) -> Option<AuthContext> {
    resolve_auth_headers(&parts.headers, state).await
}

/// Resolve auth from request headers (shared by extractors and view layers).
pub async fn resolve_auth_headers(
    headers: &axum::http::HeaderMap,
    state: &UsersState,
) -> Option<AuthContext> {
    let token = session::auth_token_from_headers(headers)?;
    let claims = jwt::parse_token(&token, &state.signing_key, &state.jwt_issuer).ok()?;
    let user_id = jwt::user_id_from_subject(&claims.sub).ok()?;
    let user = UserEntity::find_by_id(user_id).one(&state.db).await.ok()??;
    if user.deleted_at.is_some() {
        return None;
    }
    if claims.sub != jwt::subject(&user) {
        return None;
    }
    let role = auth::role_name_for_user(&state.db, &user).await.ok()?;
    Some(AuthContext {
        timezone: user.timezone.clone(),
        user,
        role,
    })
}

fn users_from_extensions(parts: &Parts) -> UsersState {
    parts
        .extensions
        .get::<UsersState>()
        .cloned()
        .unwrap_or_else(|| panic!("UsersState missing from request; is the users plugin installed?"))
}

// Optional auth extractor.
pub struct OptionalAuth(pub Option<AuthContext>);

impl<S> FromRequestParts<S> for OptionalAuth
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let users = users_from_extensions(parts);
        Ok(OptionalAuth(resolve_auth(parts, &users).await))
    }
}

// Requires authentication; redirects to login otherwise.
pub struct RequireAuth(pub AuthContext);

pub enum AuthRejection {
    Redirect(Redirect),
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        match self {
            AuthRejection::Redirect(r) => r.into_response(),
        }
    }
}

impl<S> FromRequestParts<S> for RequireAuth
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let users = users_from_extensions(parts);
        match resolve_auth(parts, &users).await {
            Some(ctx) => Ok(RequireAuth(ctx)),
            None => Err(AuthRejection::Redirect(Redirect::to("/users/login"))),
        }
    }
}

// Requires superuser or a configured staff role.
pub struct RequireStaff(pub AuthContext);

pub enum StaffRejection {
    Auth(AuthRejection),
    Forbidden,
}

impl IntoResponse for StaffRejection {
    fn into_response(self) -> Response {
        match self {
            StaffRejection::Auth(a) => a.into_response(),
            StaffRejection::Forbidden => StatusCode::UNAUTHORIZED.into_response(),
        }
    }
}

impl<S> FromRequestParts<S> for RequireStaff
where
    S: Send + Sync,
{
    type Rejection = StaffRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let RequireAuth(ctx) = RequireAuth::from_request_parts(parts, state)
            .await
            .map_err(StaffRejection::Auth)?;
        let users = users_from_extensions(parts);
        if is_staff(&ctx, &users.config.staff_roles) {
            Ok(RequireStaff(ctx))
        } else {
            Err(StaffRejection::Forbidden)
        }
    }
}

/// Requires superuser only (backward-compatible alias for routes that must stay superuser-only).
pub struct RequireSuperuser(pub AuthContext);

pub enum SuperuserRejection {
    Auth(AuthRejection),
    Forbidden,
}

impl IntoResponse for SuperuserRejection {
    fn into_response(self) -> Response {
        match self {
            SuperuserRejection::Auth(a) => a.into_response(),
            SuperuserRejection::Forbidden => StatusCode::UNAUTHORIZED.into_response(),
        }
    }
}

impl<S> FromRequestParts<S> for RequireSuperuser
where
    S: Send + Sync,
{
    type Rejection = SuperuserRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let RequireAuth(ctx) = RequireAuth::from_request_parts(parts, state)
            .await
            .map_err(SuperuserRejection::Auth)?;
        if !ctx.user.is_superuser {
            return Err(SuperuserRejection::Forbidden);
        }
        Ok(RequireSuperuser(ctx))
    }
}

pub fn is_staff(ctx: &AuthContext, staff_roles: &[String]) -> bool {
    if ctx.user.is_superuser {
        return true;
    }
    staff_roles.iter().any(|r| r == &ctx.role)
}

pub fn roles_allowed(ctx: &AuthContext, allowed: &[&str]) -> bool {
    ctx.user.is_superuser || allowed.iter().any(|r| *r == ctx.role)
}

/// Alias for staff check with explicit role slice.
pub fn staff_roles_allowed(ctx: &AuthContext, staff_roles: &[String]) -> bool {
    is_staff(ctx, staff_roles)
}

#[allow(dead_code)]
pub async fn check_roles(
    Cap(state): Cap<UsersState>,
    parts: &Parts,
    allowed: &[&str],
) -> Result<AuthContext, UsersError> {
    let ctx = resolve_auth(parts, &state)
        .await
        .ok_or(UsersError::Unauthorized)?;
    if roles_allowed(&ctx, allowed) {
        Ok(ctx)
    } else {
        Err(UsersError::Unauthorized)
    }
}

#[allow(dead_code)]
pub fn _cookie_name() -> &'static str {
    AUTH_COOKIE
}
