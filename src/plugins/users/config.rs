use base64::{Engine, engine::general_purpose::STANDARD as B64};
use rand::RngCore;
use serde::Deserialize;

use crate::config::ConfigSection;

// Config HList tag for [`UsersConfig`] (`[users]` in TOML).
pub struct UsersConfigTag;

impl ConfigSection for UsersConfigTag {
    const KEY: Option<&'static str> = Some("users");
}

// Auth / users plugin configuration (aligned with Go `AuthConfig`).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UsersConfig {
    // Base64-encoded HS512 signing key. Random if empty.
    #[serde(default, rename = "signingKey")]
    pub signing_key: String,
    // Base64-encoded JWT audience material. Random if empty.
    #[serde(default, rename = "jwtIssuer")]
    pub jwt_issuer: String,
    #[serde(default, rename = "adminEmail")]
    pub admin_email: String,
    #[serde(default, rename = "adminPassword")]
    pub admin_password: String,
    /// Roles that may access staff-only user management routes (superuser always allowed).
    #[serde(default, rename = "staffRoles")]
    pub staff_roles: Vec<String>,
}

impl UsersConfig {
    // Resolved binary signing key (config or random 64 bytes when unset).
    pub fn signing_key_bytes(&self) -> Vec<u8> {
        decode_or_random(&self.signing_key, "signingKey", 64)
    }

    // Resolved binary JWT issuer material (config or random 64 bytes when unset).
    pub fn jwt_issuer_bytes(&self) -> Vec<u8> {
        decode_or_random(&self.jwt_issuer, "jwtIssuer", 64)
    }
}

fn decode_or_random(b64: &str, field: &str, len: usize) -> Vec<u8> {
    if b64.is_empty() {
        let mut buf = vec![0u8; len];
        rand::thread_rng().fill_bytes(&mut buf);
        return buf;
    }
    B64.decode(b64).unwrap_or_else(|err| {
        panic!(
            "[users].{field} must be valid base64 when set (got {b64:?}: {err}); \
             sessions are invalidated on every restart when this value is ignored"
        );
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_signing_key_panics() {
        let cfg = UsersConfig {
            signing_key: "not!!!valid-base64".into(),
            ..Default::default()
        };
        let result = std::panic::catch_unwind(|| cfg.signing_key_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn empty_signing_key_is_randomized() {
        let a = UsersConfig::default().signing_key_bytes();
        let b = UsersConfig::default().signing_key_bytes();
        assert_eq!(a.len(), 64);
        assert_eq!(b.len(), 64);
        assert_ne!(a, b);
    }
}
