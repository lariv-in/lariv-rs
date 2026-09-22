//! Meets plugin configuration (`[meets]` in TOML).

use serde::Deserialize;

use crate::config::ConfigSection;

/// Config HList tag for [`MeetsConfig`].
pub struct MeetsConfigTag;

impl ConfigSection for MeetsConfigTag {
    const KEY: Option<&'static str> = Some("meets");
}

fn default_room_code_length() -> usize {
    8
}

fn default_transport_bind() -> String {
    "0.0.0.0:4433".into()
}

/// MoQ relay listener settings (`[meets.transport]`).
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TransportConfig {
    /// When false, the MoQ relay is not started.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// QUIC/HTTPS bind address (reverse proxy forwards HTTP/3 here).
    #[serde(default = "default_transport_bind")]
    pub bind: String,
    /// PEM certificate for the MoQ relay.
    #[serde(default, rename = "certFile")]
    pub cert_file: String,
    /// PEM private key for the MoQ relay.
    #[serde(default, rename = "keyFile")]
    pub key_file: String,
    /// Public HTTPS origin for browser MoQ URLs (no trailing slash).
    #[serde(default, rename = "publicUrl")]
    pub public_url: String,
    /// JWK file used to sign participant JWTs. Empty → ephemeral dev key.
    #[serde(default, rename = "authKeyFile")]
    pub auth_key_file: String,
}

fn default_enabled() -> bool {
    true
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            bind: default_transport_bind(),
            cert_file: String::new(),
            key_file: String::new(),
            public_url: String::new(),
            auth_key_file: String::new(),
        }
    }
}

impl TransportConfig {
    /// Browser-facing MoQ relay URL for a room (JWT travels in `?jwt=`).
    pub fn room_relay_url(&self, room_code: &str) -> String {
        let path = format!("/meets/{room_code}");
        if self.public_url.is_empty() {
            path
        } else {
            let base = self.public_url.trim_end_matches('/');
            format!("{base}{path}")
        }
    }
}

/// Room-code and MoQ relay settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MeetsConfig {
    #[serde(default)]
    pub transport: TransportConfig,
    #[serde(default = "default_room_code_length", rename = "roomCodeLength")]
    pub room_code_length: usize,
    /// HMAC-like secret for the anonymous `meets-anon` cookie. Empty → random per process.
    #[serde(default, rename = "anonCookieSecret")]
    pub anon_cookie_secret: String,
}

impl Default for MeetsConfig {
    fn default() -> Self {
        Self {
            transport: TransportConfig::default(),
            room_code_length: default_room_code_length(),
            anon_cookie_secret: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TransportConfig;

    #[test]
    fn room_relay_url_uses_public_origin() {
        let cfg = TransportConfig {
            public_url: "https://meet.example.com".into(),
            ..TransportConfig::default()
        };
        assert_eq!(
            cfg.room_relay_url("abc123"),
            "https://meet.example.com/meets/abc123"
        );
    }

    #[test]
    fn room_relay_url_relative_when_no_public_origin() {
        let cfg = TransportConfig::default();
        assert_eq!(cfg.room_relay_url("xyz"), "/meets/xyz");
    }
}
