//! Axum auth extractors — optional auth, require auth, and login redirects.
//!
//! Route allowlists live in [`super::role_authorization`].
use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::EntityTrait;

use crate::{
    auth,
    entities::user::Entity as UserEntity,
    jwt,
    roles::Superuser,
    routes::UsersLoginGetRouteTag,
    session,
    state::{AuthContext, UsersState},
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
    let user = lariv_core::web::opt_or_log(
        UserEntity::find_by_id(user_id).one(&state.db).await,
        "find user by id for auth",
    )?;
    if claims.sub != jwt::subject(&user) {
        return None;
    }
    let role = auth::role_name_for_user(&user);
    Some(AuthContext {
        timezone: user.timezone.to_string(),
        user,
        role,
    })
}

fn users_from_extensions(parts: &Parts) -> UsersState {
    parts
        .extensions
        .get::<UsersState>()
        .cloned()
        .unwrap_or_else(|| {
            panic!("UsersState missing from request; is the users plugin installed?")
        })
}

/// Optional auth extractor.
pub struct OptionalAuth(pub Option<AuthContext>);

impl<S> FromRequestParts<S> for OptionalAuth
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let users = users_from_extensions(parts);
        Ok(OptionalAuth(resolve_auth(parts, &users).await))
    }
}

/// Requires authentication; redirects to login otherwise.
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

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(ctx) = parts.extensions.get::<AuthContext>().cloned() {
            return Ok(RequireAuth(ctx));
        }
        let users = users_from_extensions(parts);
        match resolve_auth(parts, &users).await {
            Some(ctx) => Ok(RequireAuth(ctx)),
            None => Err(AuthRejection::Redirect(Redirect::to(
                &UsersLoginGetRouteTag.url(),
            ))),
        }
    }
}

/// Whether `viewer` may reset `target_user_id`'s password.
pub fn can_change_user_password(viewer: &AuthContext, target_user_id: i64) -> bool {
    Superuser::matches(&viewer.role) || viewer.user.id == target_user_id
}

/// Whether `viewer` may assign or remove the Superuser role.
pub fn can_set_superuser(viewer: &AuthContext) -> bool {
    Superuser::matches(&viewer.role)
}

pub fn roles_allowed(ctx: &AuthContext, allowed: &[&str]) -> bool {
    Superuser::matches(&ctx.role) || allowed.iter().any(|r| *r == ctx.role)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::can_change_user_password;
    use crate::{entities::user::Model as User, state::AuthContext};

    fn test_auth(id: i64, role: &str) -> AuthContext {
        AuthContext {
            user: User {
                id,
                created_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
                name: format!("User {id}"),
                email: format!("user{id}@example.com").into(),
                phone: format!("{id}").into(),
                role: role.into(),
                password_hash: Some(vec![]),
                password_salt: Some(vec![]),
                timezone: "UTC".into(),
            },
            role: role.into(),
            timezone: "UTC".into(),
        }
    }

    #[test]
    fn can_change_user_password_superuser_any_target() {
        let viewer = test_auth(1, crate::roles::Superuser::NAME);
        assert!(can_change_user_password(&viewer, 99));
    }

    #[test]
    fn can_change_user_password_only_self() {
        let viewer = test_auth(5, "admin");
        assert!(can_change_user_password(&viewer, 5));
        assert!(!can_change_user_password(&viewer, 99));
    }

    #[test]
    fn can_set_superuser_only_superuser() {
        let superuser = test_auth(1, crate::roles::Superuser::NAME);
        let other = test_auth(2, "admin");
        assert!(super::can_set_superuser(&superuser));
        assert!(!super::can_set_superuser(&other));
    }
}
