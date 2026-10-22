//! Top-bar effective-user selector. Authorization uses the logged-in user, not the
//! effective user, so a superuser can switch back after acting as someone else.

use axum::extract::Query;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use sea_orm::EntityTrait;

use crate::{
    entities::user::Entity as UserEntity,
    forms::EffectiveUserForm,
    handlers::users::{UserListQuery, render_user_select},
    role_authorization::actual_auth,
    roles::Superuser,
    session::{clear_effective_user_cookie, is_secure_request, set_effective_user_cookie},
    state::UsersState,
};
use lariv_core::components::nav_origin::DASHBOARD_URL;
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::web::Htmx;

fn require_actual_superuser() -> Result<crate::state::AuthContext, StatusCode> {
    match actual_auth() {
        Some(auth) if Superuser::matches(&auth.role) => Ok(auth),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

/// HTTP handler: user picker for the top-bar foreign-key control.
///
/// "Me" is the logged-in user, so a superuser who is acting as someone else can
/// switch back. The route is not role-gated on the effective user.
pub async fn options(
    Cap(state): Cap<UsersState>,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<UserListQuery>,
) -> Response {
    let actual = match require_actual_superuser() {
        Ok(auth) => auth,
        Err(status) => return status.into_response(),
    };
    render_user_select(
        &state,
        &htmx,
        &uri,
        &q,
        actual.user.id,
        &actual.user.name,
        &actual.timezone,
        true,
    )
    .await
    .into_response()
}

/// HTTP handler: set or clear the effective-user cookie, then open the dashboard.
pub async fn set(
    Cap(state): Cap<UsersState>,
    htmx: Htmx,
    headers: HeaderMap,
    HtmlFormBody(form): HtmlFormBody<EffectiveUserForm>,
) -> Response {
    let actual = match require_actual_superuser() {
        Ok(auth) => auth,
        Err(status) => return status.into_response(),
    };
    let secure = is_secure_request(&headers);
    let mut response = htmx.redirect(DASHBOARD_URL);
    let raw = form.user_id.trim();
    if raw.is_empty() {
        clear_effective_user_cookie(response.headers_mut(), secure);
        return response;
    }
    let Ok(id) = raw.parse::<i64>() else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if id == actual.user.id {
        clear_effective_user_cookie(response.headers_mut(), secure);
        return response;
    }
    let Some(user) = UserEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if Superuser::matches(&user.role) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    set_effective_user_cookie(response.headers_mut(), id, secure);
    response
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use axum::routing::get;
    use base64::Engine;
    use sea_orm::ActiveValue::Set;
    use sea_orm::{
        ActiveModelTrait, ConnectionTrait, Database, DatabaseBackend, EntityTrait, Schema,
    };
    use tower::ServiceExt;

    use crate::config::UsersConfig;
    use crate::entities::user::{self, Entity as UserEntity};
    use crate::role_authorization::continue_with_auth_scope;
    use crate::state::UsersState;

    const CSRF: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    async fn fixture() -> (UsersState, i64, i64) {
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
        ) -> i64 {
            let now = chrono::Utc::now();
            user::ActiveModel {
                created_at: Set(Some(now)),
                updated_at: Set(Some(now)),
                name: Set(name.to_owned()),
                email: Set(email.into()),
                phone: Set(phone.into()),
                role: Set(role.to_owned()),
                password_hash: Set(Some(vec![1, 2, 3])),
                password_salt: Set(Some(vec![salt; 32])),
                timezone: Set("UTC".into()),
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
        )
        .await;
        let clerk = insert(&db, "Bea", "bea@example.com", "2", "employee", 2).await;
        let state = UsersState::new(
            db,
            UsersConfig {
                signing_key: base64::engine::general_purpose::STANDARD.encode([6u8; 64]),
                jwt_issuer: base64::engine::general_purpose::STANDARD.encode([8u8; 64]),
                ..Default::default()
            },
        );
        (state, admin, clerk)
    }

    async fn token(state: &UsersState, user_id: i64) -> String {
        let user = UserEntity::find_by_id(user_id)
            .one(&state.db)
            .await
            .unwrap()
            .unwrap();
        crate::auth::login_token(&user, &state.signing_key, &state.jwt_issuer).unwrap()
    }

    fn app(state: UsersState) -> Router {
        let state_for_ext = state.clone();
        Router::new()
            .route(
                "/dashboard/users/effective-user",
                get(super::options).post(super::set),
            )
            .layer(axum::middleware::from_fn(continue_with_auth_scope))
            .layer(axum::middleware::from_fn(
                move |mut req: Request<Body>, next: axum::middleware::Next| {
                    let state = state_for_ext.clone();
                    async move {
                        req.extensions_mut().insert(state);
                        next.run(req).await
                    }
                },
            ))
    }

    fn cookie(token: &str, effective: Option<i64>) -> String {
        let mut cookie = format!("auth-token={token}; csrf_token={CSRF}");
        if let Some(id) = effective {
            cookie.push_str(&format!("; effective-user={id}"));
        }
        cookie
    }

    async fn call(app: &Router, request: Request<Body>) -> axum::response::Response {
        app.clone().oneshot(request).await.unwrap()
    }

    #[tokio::test]
    async fn set_effective_user_rejects_anyone_but_the_actual_superuser() {
        let (state, admin, clerk) = fixture().await;
        let router = app(state.clone());
        let admin_token = token(&state, admin).await;
        let clerk_token = token(&state, clerk).await;

        let anon = call(
            &router,
            Request::builder()
                .method("POST")
                .uri("/dashboard/users/effective-user")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, format!("csrf_token={CSRF}"))
                .body(Body::from(format!("user_id={clerk}&csrf_token={CSRF}")))
                .unwrap(),
        )
        .await;
        assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);

        let employee = call(
            &router,
            Request::builder()
                .method("POST")
                .uri("/dashboard/users/effective-user")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, cookie(&clerk_token, None))
                .body(Body::from(format!("user_id={admin}&csrf_token={CSRF}")))
                .unwrap(),
        )
        .await;
        assert_eq!(employee.status(), StatusCode::UNAUTHORIZED);
        assert!(employee.headers().get(header::SET_COOKIE).is_none());

        let switched = call(
            &router,
            Request::builder()
                .method("POST")
                .uri("/dashboard/users/effective-user")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, cookie(&admin_token, Some(clerk)))
                .body(Body::from(format!("user_id={admin}&csrf_token={CSRF}")))
                .unwrap(),
        )
        .await;
        assert_eq!(switched.status(), StatusCode::SEE_OTHER);
        let cleared = switched
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(cleared.starts_with("effective-user="), "{cleared}");
        assert!(cleared.contains("Max-Age=0"), "{cleared}");

        let missing = call(
            &router,
            Request::get("/dashboard/users/effective-user")
                .header(header::COOKIE, cookie(&clerk_token, Some(admin)))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn superuser_can_set_and_list_the_effective_user() {
        let (state, admin, clerk) = fixture().await;
        let router = app(state.clone());
        let admin_token = token(&state, admin).await;

        let set_resp = call(
            &router,
            Request::builder()
                .method("POST")
                .uri("/dashboard/users/effective-user")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, cookie(&admin_token, None))
                .body(Body::from(format!("user_id={clerk}&csrf_token={CSRF}")))
                .unwrap(),
        )
        .await;
        assert_eq!(set_resp.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            set_resp.headers().get(header::LOCATION).unwrap(),
            "/dashboard/"
        );
        let set_cookie = set_resp
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            set_cookie.starts_with(&format!("effective-user={clerk}")),
            "{set_cookie}"
        );

        let menu = call(
            &router,
            Request::get("/dashboard/users/effective-user")
                .header(header::COOKIE, cookie(&admin_token, Some(clerk)))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(menu.status(), StatusCode::OK);
        let body = axum::body::to_bytes(menu.into_body(), 64 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("Bea"), "{html}");
        assert!(html.contains("Me"), "{html}");
        assert!(html.contains("fk-select"), "{html}");
    }

    #[tokio::test]
    async fn superuser_cannot_switch_to_another_superuser() {
        let (state, admin, clerk) = fixture().await;
        let now = chrono::Utc::now();
        let other = user::ActiveModel {
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            name: Set("Cara".into()),
            email: Set("cara@example.com".into()),
            phone: Set("3".into()),
            role: Set(crate::roles::Superuser::NAME.into()),
            password_hash: Set(Some(vec![1, 2, 3])),
            password_salt: Set(Some(vec![4; 32])),
            timezone: Set("UTC".into()),
            ..Default::default()
        }
        .insert(&state.db)
        .await
        .expect("insert superuser")
        .id;
        let router = app(state.clone());
        let admin_token = token(&state, admin).await;

        let rejected = call(
            &router,
            Request::builder()
                .method("POST")
                .uri("/dashboard/users/effective-user")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, cookie(&admin_token, None))
                .body(Body::from(format!("user_id={other}&csrf_token={CSRF}")))
                .unwrap(),
        )
        .await;
        assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
        assert!(rejected.headers().get(header::SET_COOKIE).is_none());

        let menu = call(
            &router,
            Request::get("/dashboard/users/effective-user")
                .header(header::COOKIE, cookie(&admin_token, Some(clerk)))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(menu.status(), StatusCode::OK);
        let body = axum::body::to_bytes(menu.into_body(), 64 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("Bea"), "{html}");
        assert!(!html.contains("Cara"), "{html}");
    }
}
