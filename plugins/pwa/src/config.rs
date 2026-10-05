//! PWA plugin configuration (`[pwa]` in TOML).

use serde::{Deserialize, Serialize};

use lariv_core::config::ConfigSection;

/// W3C ImageResource `purpose` default (`any`).
fn default_image_purpose() -> String {
    "any".into()
}

/// Config HList tag for [`PwaConfig`].
pub struct PwaConfigTag;

impl ConfigSection for PwaConfigTag {
    const KEY: Option<&'static str> = Some("pwa");
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PwaIconConfig {
    #[serde(default)]
    pub src: String,
    #[serde(default)]
    pub sizes: String,
    #[serde(default, rename = "type", skip_serializing_if = "String::is_empty")]
    pub type_: String,
    /// Space-separated purposes (`any`, `maskable`, `monochrome`). Empty config → `any`.
    #[serde(default = "default_image_purpose")]
    pub purpose: String,
}

impl Default for PwaIconConfig {
    fn default() -> Self {
        Self {
            src: String::new(),
            sizes: String::new(),
            type_: String::new(),
            purpose: default_image_purpose(),
        }
    }
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PwaScreenshotConfig {
    #[serde(default)]
    pub src: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sizes: String,
    #[serde(default, rename = "type", skip_serializing_if = "String::is_empty")]
    pub type_: String,
    /// Space-separated purposes (`any`, `maskable`, `monochrome`). Empty config → `any`.
    #[serde(default = "default_image_purpose")]
    pub purpose: String,
    /// Screenshot form factor (`narrow` or `wide`). Empty → omitted from the manifest.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub form_factor: String,
}

impl Default for PwaScreenshotConfig {
    fn default() -> Self {
        Self {
            src: String::new(),
            sizes: String::new(),
            type_: String::new(),
            purpose: default_image_purpose(),
            form_factor: String::new(),
        }
    }
}

/// Configures `/app.webmanifest`, `/serviceworker.js`, `/offline`, `/favicon.ico`, and `/static/pwa`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PwaConfig {
    /// Optional filesystem path to a service worker JS file. Empty → default SW.
    #[serde(default, rename = "serviceWorkerPath")]
    pub service_worker_path: String,

    /// Optional view registry key for `/offline`. Empty → default HTML.
    #[serde(default, rename = "offlineViewName")]
    pub offline_view_name: String,

    /// Optional filesystem directory served under `/static/pwa`. Relative → next to binary.
    #[serde(default, rename = "staticDir")]
    pub static_dir: String,

    /// Optional filesystem path to a favicon file served at `/favicon.ico`. Empty → 404.
    #[serde(default, rename = "faviconPath")]
    pub favicon_path: String,

    #[serde(default, rename = "PWA_APP_NAME")]
    pub app_name: String,
    /// Home-screen / launcher label. Empty → [`Self::app_name`].
    #[serde(default, rename = "PWA_APP_SHORT_NAME")]
    pub app_short_name: String,
    #[serde(default, rename = "PWA_APP_DESCRIPTION")]
    pub app_description: String,
    #[serde(default, rename = "PWA_APP_THEME_COLOR")]
    pub app_theme_color: String,
    #[serde(default, rename = "PWA_APP_BACKGROUND_COLOR")]
    pub app_background_color: String,
    #[serde(default, rename = "PWA_APP_DISPLAY")]
    pub app_display: String,
    /// Ordered display-mode fallbacks applied before [`Self::app_display`].
    #[serde(default, rename = "PWA_APP_DISPLAY_OVERRIDE")]
    pub app_display_override: Vec<String>,
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

impl PwaConfig {
    /// Manifest `short_name`: configured value, or [`Self::app_name`] when unset.
    pub fn short_name(&self) -> &str {
        if self.app_short_name.is_empty() {
            self.app_name.as_str()
        } else {
            self.app_short_name.as_str()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_resource_purpose_defaults_to_any() {
        let icon: PwaIconConfig = serde_json::from_str(r#"{"src":"/icon.png"}"#).unwrap();
        assert_eq!(icon.purpose, "any");
        assert_eq!(PwaIconConfig::default().purpose, "any");

        let shot: PwaScreenshotConfig = serde_json::from_str(r#"{"src":"/shot.png"}"#).unwrap();
        assert_eq!(shot.purpose, "any");
        assert_eq!(PwaScreenshotConfig::default().purpose, "any");
    }

    #[test]
    fn image_resource_purpose_can_be_overridden() {
        let icon: PwaIconConfig =
            serde_json::from_str(r#"{"src":"/icon.png","purpose":"maskable"}"#).unwrap();
        assert_eq!(icon.purpose, "maskable");

        let shot: PwaScreenshotConfig =
            serde_json::from_str(r#"{"src":"/shot.png","purpose":"any maskable"}"#).unwrap();
        assert_eq!(shot.purpose, "any maskable");
    }

    #[test]
    fn screenshot_form_factor_is_optional() {
        let omitted: PwaScreenshotConfig = serde_json::from_str(r#"{"src":"/shot.png"}"#).unwrap();
        assert!(omitted.form_factor.is_empty());
        let json = serde_json::to_value(&omitted).unwrap();
        assert!(json.get("form_factor").is_none());

        let wide: PwaScreenshotConfig =
            serde_json::from_str(r#"{"src":"/wide.png","form_factor":"wide"}"#).unwrap();
        assert_eq!(wide.form_factor, "wide");
    }

    #[test]
    fn display_override_defaults_to_empty() {
        let cfg: PwaConfig = serde_json::from_str(r#"{"PWA_APP_NAME":"Lariv"}"#).unwrap();
        assert!(cfg.app_display_override.is_empty());

        let cfg: PwaConfig = serde_json::from_str(
            r#"{"PWA_APP_DISPLAY_OVERRIDE":["window-controls-overlay","standalone"]}"#,
        )
        .unwrap();
        assert_eq!(
            cfg.app_display_override,
            ["window-controls-overlay", "standalone"]
        );
    }
}
