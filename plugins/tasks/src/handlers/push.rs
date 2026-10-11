//! Subscribe and unsubscribe the signed-in user's browser for task push.

use axum::Json;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use lariv_core::http::Cap;
use lariv_plugin_users::middleware::RequireAuth;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use crate::entities::push_subscription::{self, Entity as PushSubscriptionEntity};
use crate::state::TasksState;

#[derive(Deserialize)]
pub struct SubscribeBody {
    endpoint: String,
    keys: SubscribeKeys,
}

#[derive(Deserialize)]
pub struct SubscribeKeys {
    p256dh: String,
    auth: String,
}

#[derive(Deserialize)]
pub struct UnsubscribeBody {
    endpoint: String,
}

/// URL-safe VAPID public key. A missing config generates and stores a key pair.
pub async fn vapid_public_key(Cap(state): Cap<TasksState>) -> Response {
    let settings = crate::push::ensure_settings(&state.db, &state.push).await;
    if !settings.enabled() {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        settings.public_key,
    )
        .into_response()
}

/// Store or reassign a browser push subscription for the signed-in user.
pub async fn subscribe(
    Cap(state): Cap<TasksState>,
    RequireAuth(ctx): RequireAuth,
    Json(body): Json<SubscribeBody>,
) -> Response {
    let settings = crate::push::ensure_settings(&state.db, &state.push).await;
    if !settings.enabled() {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let endpoint = body.endpoint.trim();
    let p256dh = body.keys.p256dh.trim();
    let auth = body.keys.auth.trim();
    if endpoint.is_empty() || p256dh.is_empty() || auth.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let now = Utc::now();
    let existing = match PushSubscriptionEntity::find()
        .filter(push_subscription::Column::Endpoint.eq(endpoint))
        .one(&state.db)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!(target: "tasks::push", "find push subscription failed: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let saved = if let Some(row) = existing {
        let mut model: push_subscription::ActiveModel = row.into();
        model.user_id = Set(ctx.user.id);
        model.p256dh = Set(p256dh.to_string());
        model.auth = Set(auth.to_string());
        model.updated_at = Set(Some(now));
        model.update(&state.db).await
    } else {
        push_subscription::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            user_id: Set(ctx.user.id),
            endpoint: Set(endpoint.to_string()),
            p256dh: Set(p256dh.to_string()),
            auth: Set(auth.to_string()),
        }
        .insert(&state.db)
        .await
    };
    if let Err(e) = saved {
        tracing::error!(target: "tasks::push", "save push subscription failed: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}

/// Drop the signed-in user's subscription for this endpoint.
pub async fn unsubscribe(
    Cap(state): Cap<TasksState>,
    RequireAuth(ctx): RequireAuth,
    Json(body): Json<UnsubscribeBody>,
) -> Response {
    let endpoint = body.endpoint.trim();
    if endpoint.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let result = PushSubscriptionEntity::delete_many()
        .filter(push_subscription::Column::Endpoint.eq(endpoint))
        .filter(push_subscription::Column::UserId.eq(ctx.user.id))
        .exec(&state.db)
        .await;
    if let Err(e) = result {
        tracing::error!(target: "tasks::push", "delete push subscription failed: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}
