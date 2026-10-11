//! Web Push payload, VAPID send, and the tasks service-worker module.
//!
//! `[tasks]` supplies the VAPID key pair. The periodic worker sends a
//! notification to the assignee after it creates a task.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Utc};
use lariv_core::datetime::{DEFAULT_TIMEZONE, DatetimeLabel};
use lariv_core::duration::format_duration;
use lariv_plugin_pwa::service_worker::{
    ServiceWorkerModule, ServiceWorkerRegistrar, ServiceWorkerRegistry,
};
use lariv_plugin_users::entities::user::Entity as UserEntity;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use serde_json::json;
use web_push::{ContentEncoding, SubscriptionInfo, VapidSignatureBuilder, WebPushMessageBuilder};

use crate::entities::{
    push_subscription::{self, Entity as PushSubscriptionEntity},
    task,
    task_status::Entity as TaskStatusEntity,
    vapid_key::{self, Entity as VapidKeyEntity},
};
use crate::routes::TaskDetailRouteTag;

const LOG_TARGET: &str = "tasks::push";
/// Encrypted payload limit enforced by the web-push crate (3052).
const MAX_PAYLOAD: usize = 3000;
const DEFAULT_VAPID_SUBJECT: &str = "mailto:tasks@localhost";
const VAPID_SINGLETON: i32 = 1;

static DISABLED_WARNED: AtomicBool = AtomicBool::new(false);

/// VAPID material copied onto [`crate::state::TasksState`] at mount.
#[derive(Clone, Debug, Default)]
pub struct PushSettings {
    pub public_key: String,
    pub private_key: String,
    pub subject: String,
}

impl PushSettings {
    pub fn from_config(config: &crate::config::TasksConfig) -> Self {
        Self {
            public_key: config.vapid_public_key.trim().to_string(),
            private_key: config.vapid_private_key.trim().to_string(),
            subject: config.vapid_subject.trim().to_string(),
        }
    }

    /// True when a public key, private key, and subject are all set.
    pub fn enabled(&self) -> bool {
        !self.public_key.is_empty() && !self.private_key.is_empty() && !self.subject.is_empty()
    }
}

/// Configured keys, or a pair generated once and stored in `task_vapid_keys`.
///
/// The lock is shared with every request. A later call sees the same pair, including
/// one generated in memory when the table is not migrated yet.
pub async fn ensure_settings(
    db: &DatabaseConnection,
    slot: &Arc<RwLock<PushSettings>>,
) -> PushSettings {
    {
        let current = read_settings(slot);
        if current.enabled() {
            return current;
        }
    }
    if let Some(stored) = load_stored(db).await {
        write_settings(slot, stored.clone());
        return stored;
    }
    let subject = read_settings(slot).subject;
    let generated = match generate_settings(&subject) {
        Ok(settings) => settings,
        Err(e) => {
            tracing::error!(target: LOG_TARGET, "generate vapid key failed: {e}");
            return read_settings(slot);
        }
    };
    write_settings(slot, generated.clone());
    match save_stored(db, &generated).await {
        Ok(()) => generated,
        Err(e) => {
            tracing::error!(
                target: LOG_TARGET,
                "save vapid key failed (run migrations so it survives restart): {e}"
            );
            if let Some(stored) = load_stored(db).await {
                write_settings(slot, stored.clone());
                stored
            } else {
                generated
            }
        }
    }
}

fn read_settings(slot: &Arc<RwLock<PushSettings>>) -> PushSettings {
    slot.read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

fn write_settings(slot: &Arc<RwLock<PushSettings>>, settings: PushSettings) {
    *slot
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = settings;
}

fn generate_settings(subject: &str) -> Result<PushSettings, String> {
    let key = jwt_simple::prelude::ES256KeyPair::generate();
    let private_key = URL_SAFE_NO_PAD.encode(key.to_bytes());
    let public_key = URL_SAFE_NO_PAD.encode(
        VapidSignatureBuilder::from_base64_no_sub(&private_key)
            .map_err(|e| e.to_string())?
            .get_public_key(),
    );
    let subject = if subject.trim().is_empty() {
        DEFAULT_VAPID_SUBJECT.to_string()
    } else {
        subject.trim().to_string()
    };
    Ok(PushSettings {
        public_key,
        private_key,
        subject,
    })
}

async fn load_stored(db: &DatabaseConnection) -> Option<PushSettings> {
    match VapidKeyEntity::find().one(db).await {
        Ok(Some(row)) => {
            let settings = PushSettings {
                public_key: row.public_key,
                private_key: row.private_key,
                subject: row.subject,
            };
            settings.enabled().then_some(settings)
        }
        Ok(None) => None,
        Err(e) => {
            tracing::error!(target: LOG_TARGET, "load vapid key failed: {e}");
            None
        }
    }
}

async fn save_stored(db: &DatabaseConnection, settings: &PushSettings) -> Result<(), String> {
    let now = Utc::now();
    vapid_key::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        singleton: Set(VAPID_SINGLETON),
        public_key: Set(settings.public_key.clone()),
        private_key: Set(settings.private_key.clone()),
        subject: Set(settings.subject.clone()),
    }
    .insert(db)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Log once when push is not configured.
pub fn warn_if_disabled(push: &PushSettings) {
    if push.enabled() {
        return;
    }
    if DISABLED_WARNED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    tracing::warn!(
        target: LOG_TARGET,
        "task push disabled; set [tasks] vapidPublicKey, vapidPrivateKey, and vapidSubject"
    );
}

/// Registers [`crate::routes::TasksPushServiceWorkerRouteTag`].
#[derive(Clone, Copy, Default)]
pub struct ServiceWorkerHook;

impl ServiceWorkerRegistrar for ServiceWorkerHook {
    fn register_service_workers(self, registry: ServiceWorkerRegistry) -> ServiceWorkerRegistry {
        registry.register(ServiceWorkerModule::<
            crate::routes::TasksPushServiceWorkerRouteTag,
        >::new(include_str!("push_sw.js")))
    }
}

/// Notify `task.assigned_to_id` using the shared VAPID settings.
///
/// A missing subscription or a failed send is logged and does not fail the caller.
pub async fn notify_assignee(
    db: &DatabaseConnection,
    slot: &Arc<RwLock<PushSettings>>,
    task: &task::Model,
) {
    let settings = ensure_settings(db, slot).await;
    notify_created(db, &settings, http_client(), task).await;
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

/// Notify the assignee. A missing subscription or a failed send is logged and does not fail the caller.
pub async fn notify_created(
    db: &DatabaseConnection,
    push: &PushSettings,
    client: &reqwest::Client,
    task: &task::Model,
) {
    if !push.enabled() {
        return;
    }
    let status_name = match TaskStatusEntity::find_by_id(task.status_id).one(db).await {
        Ok(Some(status)) => status.name,
        Ok(None) => "Unknown".to_string(),
        Err(e) => {
            tracing::error!(target: LOG_TARGET, "load task status failed: {e}");
            return;
        }
    };
    let timezone = match UserEntity::find_by_id(task.assigned_to_id).one(db).await {
        Ok(Some(user)) => {
            let tz = user.timezone.as_str().trim();
            if tz.is_empty() {
                DEFAULT_TIMEZONE.to_string()
            } else {
                tz.to_string()
            }
        }
        Ok(None) => DEFAULT_TIMEZONE.to_string(),
        Err(e) => {
            tracing::error!(target: LOG_TARGET, "load assignee failed: {e}");
            return;
        }
    };
    let payload = notification_payload(
        &task.title,
        &task.description,
        task.due_datetime,
        Utc::now(),
        &timezone,
        &status_name,
        &TaskDetailRouteTag::new(task.id).path(),
    );
    let subs = match PushSubscriptionEntity::find()
        .filter(push_subscription::Column::UserId.eq(task.assigned_to_id))
        .all(db)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(target: LOG_TARGET, "load push subscriptions failed: {e}");
            return;
        }
    };
    for sub in subs {
        match send_one(
            client,
            push,
            &sub.endpoint,
            &sub.p256dh,
            &sub.auth,
            &payload,
        )
        .await
        {
            Ok(SendOutcome::Sent) => {}
            Ok(SendOutcome::Gone) => {
                if let Err(e) = PushSubscriptionEntity::delete_by_id(sub.id).exec(db).await {
                    tracing::error!(
                        target: LOG_TARGET,
                        subscription_id = sub.id,
                        "drop expired push subscription failed: {e}"
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    target: LOG_TARGET,
                    subscription_id = sub.id,
                    "push send failed: {e}"
                );
            }
        }
    }
}

enum SendOutcome {
    Sent,
    Gone,
}

async fn send_one(
    client: &reqwest::Client,
    push: &PushSettings,
    endpoint: &str,
    p256dh: &str,
    auth: &str,
    payload: &[u8],
) -> Result<SendOutcome, String> {
    let info = SubscriptionInfo::new(endpoint, p256dh, auth);
    let mut sig = vapid_builder(&push.private_key, &info)?;
    sig.add_claim("sub", push.subject.as_str());
    let signature = sig.build().map_err(|e| e.to_string())?;
    let mut builder = WebPushMessageBuilder::new(&info);
    builder.set_payload(ContentEncoding::Aes128Gcm, payload);
    builder.set_vapid_signature(signature);
    let message = builder.build().map_err(|e| e.to_string())?;
    let mut req = client
        .post(message.endpoint.to_string())
        .header("TTL", message.ttl.to_string());
    if let Some(body) = message.payload {
        req = req
            .header("Content-Encoding", body.content_encoding.to_str())
            .header("Content-Type", "application/octet-stream");
        for (name, value) in body.crypto_headers {
            req = req.header(name, value);
        }
        req = req.body(body.content);
    }
    let response = req.send().await.map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    if status == 404 || status == 410 {
        return Ok(SendOutcome::Gone);
    }
    if (200..300).contains(&status) {
        return Ok(SendOutcome::Sent);
    }
    let text = response.text().await.unwrap_or_default();
    Err(format!("status {status}: {text}"))
}

fn vapid_builder<'a>(
    private_key: &str,
    info: &'a SubscriptionInfo,
) -> Result<VapidSignatureBuilder<'a>, String> {
    if private_key.contains("BEGIN") {
        VapidSignatureBuilder::from_pem(private_key.as_bytes(), info).map_err(|e| e.to_string())
    } else {
        VapidSignatureBuilder::from_base64(private_key, info).map_err(|e| e.to_string())
    }
}

/// JSON body shown by the service worker. Description is trimmed to stay under the push limit.
pub fn notification_payload(
    title: &str,
    description: &str,
    due: DateTime<Utc>,
    now: DateTime<Utc>,
    timezone: &str,
    status: &str,
    url: &str,
) -> Vec<u8> {
    let due_label = DatetimeLabel::short(due, timezone).into_string();
    let until = time_until_due(due, now);
    let mut description = description.to_string();
    loop {
        let body = notification_body(&description, &due_label, &until, status);
        let bytes = serde_json::to_vec(&json!({
            "title": title,
            "body": body,
            "url": url,
        }))
        .unwrap_or_default();
        if bytes.len() <= MAX_PAYLOAD || description.is_empty() {
            return bytes;
        }
        let next = description.len().saturating_sub(64);
        let end = floor_char_boundary(&description, next);
        description.truncate(end);
        description = description.trim_end().to_string();
        if !description.is_empty() {
            description.push('…');
        }
    }
}

fn notification_body(description: &str, due_label: &str, until: &str, status: &str) -> String {
    let mut lines = Vec::new();
    if !description.is_empty() {
        lines.push(description.to_string());
        lines.push(String::new());
    }
    lines.push(format!("Due: {due_label}"));
    lines.push(until.to_string());
    lines.push(format!("Status: {status}"));
    lines.join("\n")
}

/// `Due in …`, `Due now`, or `Overdue by …`.
pub fn time_until_due(due: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let delta = due.signed_duration_since(now);
    let nanos = delta
        .num_nanoseconds()
        .unwrap_or_else(|| delta.num_seconds().saturating_mul(1_000_000_000));
    if nanos > 0 {
        format!("Due in {}", format_duration(nanos))
    } else if nanos < 0 {
        let overdue = format_duration(nanos.saturating_neg());
        if overdue.is_empty() {
            "Overdue".to_string()
        } else {
            format!("Overdue by {overdue}")
        }
    } else {
        "Due now".to_string()
    }
}

fn floor_char_boundary(s: &str, mut index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    while index > 0 && !s.is_char_boundary(index) {
        index -= 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use base64::Engine;

    use super::*;

    fn ts(rfc3339: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(rfc3339)
            .expect("rfc3339")
            .with_timezone(&Utc)
    }

    #[test]
    fn push_settings_require_all_three_fields() {
        let empty = PushSettings::default();
        assert!(!empty.enabled());
        let partial = PushSettings {
            public_key: "pub".into(),
            private_key: "priv".into(),
            subject: String::new(),
        };
        assert!(!partial.enabled());
        let full = PushSettings {
            public_key: "pub".into(),
            private_key: "priv".into(),
            subject: "mailto:tasks@example.com".into(),
        };
        assert!(full.enabled());
    }

    #[test]
    fn generated_vapid_key_is_usable() {
        let settings = generate_settings("").expect("generate");
        assert!(settings.enabled());
        assert_eq!(settings.subject, "mailto:tasks@localhost");
        let public = URL_SAFE_NO_PAD
            .decode(settings.public_key.as_bytes())
            .expect("public key");
        assert_eq!(public.len(), 65);
        assert_eq!(public.first().copied(), Some(0x04));
        VapidSignatureBuilder::from_base64_no_sub(&settings.private_key).expect("private key");
    }

    #[test]
    fn time_until_due_labels_future_and_past() {
        let now = ts("2026-10-11T10:00:00Z");
        let later = ts("2026-10-11T12:00:00Z");
        assert_eq!(time_until_due(later, now), "Due in 2 hours");
        assert_eq!(time_until_due(now, later), "Overdue by 2 hours");
        assert_eq!(time_until_due(now, now), "Due now");
    }

    #[test]
    fn payload_includes_task_fields_and_stays_under_the_limit() {
        let due = ts("2026-10-12T18:30:00Z");
        let now = ts("2026-10-11T18:30:00Z");
        let bytes = notification_payload(
            "Water plants",
            "Check the balcony pots",
            due,
            now,
            "UTC",
            "To Do",
            "/dashboard/tasks/9",
        );
        assert!(bytes.len() <= MAX_PAYLOAD);
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(value["title"], "Water plants");
        assert_eq!(value["url"], "/dashboard/tasks/9");
        let body = value["body"].as_str().expect("body");
        assert!(body.contains("Check the balcony pots"));
        assert!(body.contains("Due:"));
        assert!(body.contains("Due in 1 day"));
        assert!(body.contains("Status: To Do"));
    }

    #[test]
    fn long_description_is_truncated() {
        let due = ts("2026-10-12T18:30:00Z");
        let now = ts("2026-10-11T18:30:00Z");
        let description = "x".repeat(8_000);
        let bytes = notification_payload(
            "Title",
            &description,
            due,
            now,
            "UTC",
            "To Do",
            "/dashboard/tasks/1",
        );
        assert!(bytes.len() <= MAX_PAYLOAD);
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        let body = value["body"].as_str().expect("body");
        assert!(body.contains('…'));
        assert!(body.contains("Status: To Do"));
    }
}
