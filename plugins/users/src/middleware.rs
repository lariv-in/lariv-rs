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
    state::{AuthContext, RequestAuth, UsersState},
};

/// Build the principal stored for a user row.
pub fn auth_context_from_user(user: crate::entities::User) -> AuthContext {
    let role = auth::role_name_for_user(&user);
    let timezone = user.timezone.to_string();
    AuthContext {
        timezone,
        user,
        role,
    }
}

/// Resolve the logged-in user from the auth cookie.
///
/// This does not apply the effective-user cookie. Request handlers see
/// [`resolve_request_auth`] instead.
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
    Some(auth_context_from_user(user))
}

/// Logged-in user plus the user this request acts as.
///
/// A non-superuser always acts as themselves. A superuser acts as the user id in
/// the effective-user cookie when that row still exists and is not a superuser,
/// and otherwise as themselves.
pub async fn resolve_request_auth(
    headers: &axum::http::HeaderMap,
    state: &UsersState,
) -> Option<RequestAuth> {
    let actual = resolve_auth_headers(headers, state).await?;
    let effective = resolve_effective_user(&actual, headers, state).await;
    Some(RequestAuth { actual, effective })
}

async fn resolve_effective_user(
    actual: &AuthContext,
    headers: &axum::http::HeaderMap,
    state: &UsersState,
) -> AuthContext {
    if !Superuser::matches(&actual.role) {
        return actual.clone();
    }
    let Some(raw) = session::effective_user_from_headers(headers) else {
        return actual.clone();
    };
    let Ok(id) = raw.trim().parse::<i64>() else {
        return actual.clone();
    };
    if id == actual.user.id {
        return actual.clone();
    }
    let Some(user) = lariv_core::web::opt_or_log(
        UserEntity::find_by_id(id).one(&state.db).await,
        "find effective user by id",
    ) else {
        return actual.clone();
    };
    if Superuser::matches(&user.role) {
        return actual.clone();
    }
    auth_context_from_user(user)
}

/// Effective user already stored on the request, or resolved from cookies.
async fn effective_from_parts(parts: &Parts, state: &UsersState) -> Option<AuthContext> {
    if let Some(ctx) = parts.extensions.get::<AuthContext>().cloned() {
        return Some(ctx);
    }
    resolve_request_auth(&parts.headers, state)
        .await
        .map(|resolved| resolved.effective)
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
        Ok(OptionalAuth(effective_from_parts(parts, &users).await))
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
        let users = users_from_extensions(parts);
        match effective_from_parts(parts, &users).await {
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
                last_login_time: None,
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

    async fn fixture() -> (crate::state::UsersState, i64, i64) {
        use base64::Engine;
        use sea_orm::ActiveValue::Set;
        use sea_orm::{ActiveModelTrait, ConnectionTrait, Database, DatabaseBackend, Schema};

        use crate::config::UsersConfig;
        use crate::entities::user::{self, Entity as UserEntity};
        use crate::state::UsersState;

        let db = Database::connect("sqlite::memory:").await.expect("sqlite");
        let stmt = Schema::new(DatabaseBackend::Sqlite).create_table_from_entity(UserEntity);
        db.execute(&stmt).await.expect("create users");

        async fn insert(
            db: &sea_orm::DatabaseConnection,
            name: &str,
            email: &str,
            phone: &str,
            role: &str,
            salt: u8,
            timezone: &str,
        ) -> i64 {
            let now = Utc::now();
            user::ActiveModel {
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                name: Set(name.to_owned()),
                email: Set(email.into()),
                phone: Set(phone.into()),
                role: Set(role.to_owned()),
                password_hash: Set(Some(vec![1, 2, 3])),
                password_salt: Set(Some(vec![salt; 32])),
                timezone: Set(timezone.into()),
                ..Default::default()
            }
            .insert(db)
            .await
            .expect("insert user")
            .id
        }

        let admin = insert(
            &db,
            "Ada",
            "ada@example.com",
            "1",
            crate::roles::Superuser::NAME,
            1,
            "UTC",
        )
        .await;
        let clerk = insert(
            &db,
            "Bea",
            "bea@example.com",
            "2",
            "employee",
            2,
            "Asia/Kolkata",
        )
        .await;
        let state = UsersState::new(
            db,
            UsersConfig {
                signing_key: base64::engine::general_purpose::STANDARD.encode([4u8; 64]),
                jwt_issuer: base64::engine::general_purpose::STANDARD.encode([5u8; 64]),
                ..Default::default()
            },
        );
        (state, admin, clerk)
    }

    async fn token_for(state: &crate::state::UsersState, user_id: i64) -> String {
        use sea_orm::EntityTrait;

        use crate::auth;
        use crate::entities::user::Entity as UserEntity;

        let user = UserEntity::find_by_id(user_id)
            .one(&state.db)
            .await
            .unwrap()
            .unwrap();
        auth::login_token(&user, &state.signing_key, &state.jwt_issuer).expect("token")
    }

    fn cookie_headers(token: &str, effective: Option<&str>) -> axum::http::HeaderMap {
        let mut cookie = format!("auth-token={token}");
        if let Some(effective) = effective {
            cookie.push_str("; effective-user=");
            cookie.push_str(effective);
        }
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::COOKIE,
            axum::http::HeaderValue::from_str(&cookie).unwrap(),
        );
        headers
    }

    #[tokio::test]
    async fn effective_user_cookie_is_ignored_for_non_superusers() {
        let (state, _admin, clerk) = fixture().await;
        let token = token_for(&state, clerk).await;
        let headers = cookie_headers(&token, Some("1"));
        let resolved = super::resolve_request_auth(&headers, &state)
            .await
            .expect("auth");
        assert_eq!(resolved.actual.user.id, clerk);
        assert_eq!(resolved.effective.user.id, clerk);
        assert_eq!(resolved.effective.role, "employee");
    }

    #[tokio::test]
    async fn superuser_effective_user_follows_the_cookie() {
        let (state, admin, clerk) = fixture().await;
        let token = token_for(&state, admin).await;
        let headers = cookie_headers(&token, Some(&clerk.to_string()));
        let resolved = super::resolve_request_auth(&headers, &state)
            .await
            .expect("auth");
        assert_eq!(resolved.actual.user.id, admin);
        assert_eq!(resolved.actual.role, crate::roles::Superuser::NAME);
        assert_eq!(resolved.effective.user.id, clerk);
        assert_eq!(resolved.effective.user.name, "Bea");
        assert_eq!(resolved.effective.role, "employee");
        assert_eq!(resolved.effective.timezone, "Asia/Kolkata");
    }

    #[tokio::test]
    async fn invalid_effective_user_falls_back_to_the_actual_user() {
        let (state, admin, _clerk) = fixture().await;
        let token = token_for(&state, admin).await;
        for raw in ["nope", "999999", ""] {
            let headers = cookie_headers(&token, Some(raw));
            let resolved = super::resolve_request_auth(&headers, &state)
                .await
                .expect("auth");
            assert_eq!(resolved.effective.user.id, admin, "{raw}");
            assert_eq!(resolved.effective.role, crate::roles::Superuser::NAME);
        }
        let headers = cookie_headers(&token, None);
        let resolved = super::resolve_request_auth(&headers, &state)
            .await
            .expect("auth");
        assert_eq!(resolved.effective.user.id, admin);
    }

    #[tokio::test]
    async fn superuser_cookie_for_another_superuser_is_ignored() {
        use sea_orm::ActiveModelTrait;
        use sea_orm::ActiveValue::Set;

        use crate::entities::user;

        let (state, admin, _clerk) = fixture().await;
        let now = Utc::now();
        let other = user::ActiveModel {
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            name: Set("Cara".into()),
            email: Set("cara@example.com".into()),
            phone: Set("3".into()),
            role: Set(crate::roles::Superuser::NAME.into()),
            password_hash: Set(Some(vec![1, 2, 3])),
            password_salt: Set(Some(vec![3; 32])),
            timezone: Set("UTC".into()),
            ..Default::default()
        }
        .insert(&state.db)
        .await
        .expect("insert superuser")
        .id;
        let token = token_for(&state, admin).await;
        let headers = cookie_headers(&token, Some(&other.to_string()));
        let resolved = super::resolve_request_auth(&headers, &state)
            .await
            .expect("auth");
        assert_eq!(resolved.actual.user.id, admin);
        assert_eq!(resolved.effective.user.id, admin);
        assert_eq!(resolved.effective.role, crate::roles::Superuser::NAME);
    }
}
