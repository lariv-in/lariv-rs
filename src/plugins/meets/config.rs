//! Meets plugin configuration (`[meets]` in TOML).

use serde::Deserialize;

use crate::config::ConfigSection;

/// Config HList tag for [`MeetsConfig`].
pub struct MeetsConfigTag;

impl ConfigSection for MeetsConfigTag {
    const KEY: Option<&'static str> = Some("meets");
}

fn default_ice_udp_port() -> u16 {
    3478
}

fn default_bind_host() -> String {
    "0.0.0.0".into()
}

fn default_stun_servers() -> Vec<String> {
    vec!["stun:stun.l.google.com:19302".into()]
}

fn default_room_code_length() -> usize {
    8
}

/// Optional TURN server from `[meets.turnServers]`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TurnServer {
    #[serde(default)]
    pub urls: Vec<String>,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub credential: String,
}

/// ICE / room-code settings for the SFU.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MeetsConfig {
    #[serde(default = "default_bind_host", rename = "bindHost")]
    pub bind_host: String,
    #[serde(default = "default_ice_udp_port", rename = "iceUdpPort")]
    pub ice_udp_port: u16,
    #[serde(default, rename = "advertisedIp")]
    pub advertised_ip: Option<String>,
    #[serde(default = "default_stun_servers", rename = "stunServers")]
    pub stun_servers: Vec<String>,
    #[serde(default, rename = "turnServers")]
    pub turn_servers: Vec<TurnServer>,
    #[serde(default = "default_room_code_length", rename = "roomCodeLength")]
    pub room_code_length: usize,
    /// HMAC-like secret for the anonymous `meets-anon` cookie. Empty → random per process.
    #[serde(default, rename = "anonCookieSecret")]
    pub anon_cookie_secret: String,
}

impl Default for MeetsConfig {
    fn default() -> Self {
        Self {
            bind_host: default_bind_host(),
            ice_udp_port: default_ice_udp_port(),
            advertised_ip: None,
            stun_servers: default_stun_servers(),
            turn_servers: Vec::new(),
            room_code_length: default_room_code_length(),
            anon_cookie_secret: String::new(),
        }
    }
}

impl MeetsConfig {
    /// JSON array of RTCIceServer dicts for the browser `RTCPeerConnection`.
    pub fn ice_servers_json(&self) -> String {
        let mut servers: Vec<serde_json::Value> = self
            .stun_servers
            .iter()
            .map(|url| serde_json::json!({ "urls": url }))
            .collect();
        for turn in &self.turn_servers {
            if turn.urls.is_empty() {
                continue;
            }
            servers.push(serde_json::json!({
                "urls": turn.urls,
                "username": turn.username,
                "credential": turn.credential,
            }));
        }
        serde_json::to_string(&servers).unwrap_or_else(|_| "[]".into())
    }
}
