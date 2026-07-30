//! PWA plugin configuration (`[p_pwa]` in TOML), aligned with Go `p_pwa.PwaConfig`.

use serde::{Deserialize, Serialize};

use crate::config::ConfigSection;

/// Config HList tag for [`PwaConfig`].
pub struct PwaConfigTag;

impl ConfigSection for PwaConfigTag {
    const KEY: Option<&'static str> = Some("p_pwa");
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PwaIconConfig {
    #[serde(default)]
    pub src: String,
    #[serde(default)]
    pub sizes: String,
    #[serde(default, rename = "type", skip_serializing_if = "String::is_empty")]
    pub type_: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PwaAppleIconConfig {
    #[serde(default)]
    pub src: String,
    #[serde(default)]
    pub sizes: String,
    #[serde(default, rename = "type", skip_serializing_if = "String::is_empty")]
    pub type_: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PwaSplashScreenConfig {
    #[serde(default)]
    pub src: String,
    #[serde(default)]
    pub media: String,
    #[serde(default, rename = "type", skip_serializing_if = "String::is_empty")]
    pub type_: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sizes: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PwaShortcutConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PwaScreenshotConfig {
    #[serde(default)]
    pub src: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sizes: String,
    #[serde(default, rename = "type", skip_serializing_if = "String::is_empty")]
    pub type_: String,
}

/// Configures `/app.webmanifest`, `/serviceworker.js`, `/offline`, and `/static/pwa`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PwaConfig {
    /// Optional filesystem path to a service worker JS file. Empty → default SW.
    #[serde(default, rename = "serviceWorkerPath")]
    pub service_worker_path: String,

    /// Optional view registry key for `/offline` (Go `NewDynamicView`). Empty → default HTML.
    #[serde(default, rename = "offlineViewName")]
    pub offline_view_name: String,

    /// Optional filesystem directory served under `/static/pwa`. Relative → next to binary.
    #[serde(default, rename = "staticDir")]
    pub static_dir: String,

    #[serde(default, rename = "PWA_APP_NAME")]
    pub app_name: String,
    #[serde(default, rename = "PWA_APP_DESCRIPTION")]
    pub app_description: String,
    #[serde(default, rename = "PWA_APP_THEME_COLOR")]
    pub app_theme_color: String,
    #[serde(default, rename = "PWA_APP_BACKGROUND_COLOR")]
    pub app_background_color: String,
    #[serde(default, rename = "PWA_APP_DISPLAY")]
    pub app_display: String,
    #[serde(default, rename = "PWA_APP_SCOPE")]
    pub app_scope: String,
    #[serde(default, rename = "PWA_APP_ORIENTATION")]
    pub app_orientation: String,
    #[serde(default, rename = "PWA_APP_START_URL")]
    pub app_start_url: String,
    #[serde(default, rename = "PWA_APP_PACKAGE_NAME")]
    pub app_package_name: String,
    #[serde(default, rename = "PWA_APP_SHA256_CERT_FINGERPRINTS")]
    pub app_sha256_cert_fingerprints: String,
    #[serde(default, rename = "PWA_APP_STATUS_BAR_COLOR")]
    pub app_status_bar_color: String,
    #[serde(default, rename = "PWA_APP_ICONS")]
    pub app_icons: Vec<PwaIconConfig>,
    #[serde(default, rename = "PWA_APP_ICONS_APPLE")]
    pub app_icons_apple: Vec<PwaAppleIconConfig>,
    #[serde(default, rename = "PWA_APP_SPLASH_SCREEN")]
    pub app_splash_screen: Vec<PwaSplashScreenConfig>,
    #[serde(default, rename = "PWA_APP_DIR")]
    pub app_dir: String,
    #[serde(default, rename = "PWA_APP_LANG")]
    pub app_lang: String,
    #[serde(default, rename = "PWA_APP_SHORTCUTS")]
    pub app_shortcuts: Vec<PwaShortcutConfig>,
    #[serde(default, rename = "PWA_APP_SCREENSHOTS")]
    pub app_screenshots: Vec<PwaScreenshotConfig>,
}
