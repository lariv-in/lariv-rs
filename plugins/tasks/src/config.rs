//! Tasks plugin configuration (`[tasks]` in TOML).

use serde::Deserialize;

use lariv_core::config::ConfigSection;

/// Config HList tag for [`TasksConfig`].
pub struct TasksConfigTag;

impl ConfigSection for TasksConfigTag {
    const KEY: Option<&'static str> = Some("tasks");
}

/// Web Push (VAPID) settings. Empty keys disable push.
///
/// The private key is either a URL-safe unpadded base64 raw P-256 scalar, or a
/// PEM (`BEGIN PRIVATE KEY` / `BEGIN EC PRIVATE KEY`). The public key is the
/// URL-safe base64 uncompressed point the browser passes to `pushManager.subscribe`.
/// When all three fields are empty, the server generates a key pair on first use
/// and stores it in `task_vapid_keys` (run migrations so it survives restart).
/// Generate a pair with:
///
/// ```text
/// openssl ecparam -name prime256v1 -genkey -noout -out private.pem
/// openssl ec -in private.pem -pubout -outform DER | tail -c 65 | base64 | tr '/+' '_-' | tr -d '\n='
/// ```
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TasksConfig {
    /// Browser `applicationServerKey` (URL-safe base64, no padding).
    #[serde(default, rename = "vapidPublicKey")]
    pub vapid_public_key: String,
    /// Server signing key. PEM or URL-safe base64 raw private scalar.
    #[serde(default, rename = "vapidPrivateKey")]
    pub vapid_private_key: String,
    /// VAPID `sub` claim (`mailto:…` or an `https:` origin).
    #[serde(default, rename = "vapidSubject")]
    pub vapid_subject: String,
}
