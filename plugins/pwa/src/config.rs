//! PWA plugin configuration (`[pwa]` in TOML).

use serde::{Deserialize, Serialize};

use lariv_core::config::ConfigSection;

/// W3C ImageResource `purpose` default (`any`).
fn default_image_purpose() -> String {
    "any".into()
}

/// Config HList tag for [`PwaConfigSet`].
pub struct PwaConfigTag;

impl ConfigSection for PwaConfigTag {
    const KEY: Option<&'static str> = Some("pwa");
}

/// Normalize a scope to a leading-slash path prefix (no trailing slash except `/`).
pub fn normalize_scope(scope: &str) -> String {
    let trimmed = scope.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return "/".into();
    }
    let with_slash = if trimmed.starts_with('/') {
        trimmed.to_owned()
    } else {
        format!("/{trimmed}")
    };
    with_slash
        .trim_end_matches('/')
        .to_owned()
}

/// True when `path` is inside `scope` (normalized). `/` matches every path.
pub fn scope_matches(path: &str, scope: &str) -> bool {
    let scope = normalize_scope(scope);
    if scope == "/" {
        return true;
    }
    let path = if path.is_empty() { "/" } else { path };
    path == scope || path.starts_with(&format!("{scope}/"))
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

/// Configures `/app.webmanifest`, `/offline`, `/favicon.ico`, and `/static/pwa`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PwaConfig {
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

    /// Normalized [`Self::app_scope`] (`/` when empty).
    pub fn scope(&self) -> String {
        normalize_scope(&self.app_scope)
    }
}

/// `[pwa]` section: root app fields plus optional `[[pwa.apps]]` extras.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PwaConfigSet {
    #[serde(flatten)]
    pub root: PwaConfig,
    #[serde(default)]
    pub apps: Vec<PwaConfig>,
}

impl PwaConfigSet {
    /// Deduplicate extra-app scopes (first wins) and warn on collisions.
    pub fn sanitize(mut self) -> Self {
        let mut seen = std::collections::HashSet::new();
        let mut unique = Vec::with_capacity(self.apps.len());
        for app in self.apps.drain(..) {
            let scope = app.scope();
            if !seen.insert(scope.clone()) {
                tracing::warn!(
                    scope = %scope,
                    "p_pwa: duplicate PWA_APP_SCOPE in [[pwa.apps]]; keeping the first"
                );
                continue;
            }
            unique.push(app);
        }
        self.apps = unique;
        self
    }

    /// Longest matching scope for `request_path`, or the root app when nothing matches.
    pub fn resolve(&self, request_path: &str) -> &PwaConfig {
        let mut best: Option<&PwaConfig> = None;
        let mut best_len = 0usize;

        let candidates = std::iter::once(&self.root).chain(self.apps.iter());
        for app in candidates {
            let scope = app.scope();
            if !scope_matches(request_path, &scope) {
                continue;
            }
            let len = if scope == "/" { 0 } else { scope.len() };
            if best.is_none() || len > best_len {
                best = Some(app);
                best_len = len;
            }
        }

        best.unwrap_or(&self.root)
    }

    /// Resolve by an explicit `scope` query (manifest/favicon/offline/static).
    ///
    /// Unknown or empty scope falls back to the root app.
    pub fn resolve_scope_query(&self, scope: Option<&str>) -> &PwaConfig {
        let Some(raw) = scope.filter(|s| !s.is_empty()) else {
            return &self.root;
        };
        let want = normalize_scope(raw);
        if self.root.scope() == want {
            return &self.root;
        }
        self.apps
            .iter()
            .find(|app| app.scope() == want)
            .unwrap_or(&self.root)
    }

    /// True when extra path-scoped apps are configured.
    pub fn has_extra_apps(&self) -> bool {
        !self.apps.is_empty()
    }

    /// Root plus every extra app (registration / asset-links order).
    pub fn all_apps(&self) -> impl Iterator<Item = &PwaConfig> {
        std::iter::once(&self.root).chain(self.apps.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_scope_trims_and_prefixes() {
        assert_eq!(normalize_scope(""), "/");
        assert_eq!(normalize_scope("/"), "/");
        assert_eq!(normalize_scope("kiosk"), "/kiosk");
        assert_eq!(normalize_scope("/kiosk/"), "/kiosk");
        assert_eq!(normalize_scope("/kiosk/orders/"), "/kiosk/orders");
    }

    #[test]
    fn scope_matches_prefix_boundaries() {
        assert!(scope_matches("/anything", "/"));
        assert!(scope_matches("/kiosk", "/kiosk"));
        assert!(scope_matches("/kiosk/", "/kiosk"));
        assert!(scope_matches("/kiosk/orders", "/kiosk"));
        assert!(!scope_matches("/kioskette", "/kiosk"));
        assert!(!scope_matches("/other", "/kiosk"));
    }

    #[test]
    fn resolve_picks_longest_matching_scope() {
        let set = PwaConfigSet {
            root: PwaConfig {
                app_name: "Root".into(),
                app_scope: "/".into(),
                ..Default::default()
            },
            apps: vec![
                PwaConfig {
                    app_name: "Kiosk".into(),
                    app_scope: "/kiosk/".into(),
                    ..Default::default()
                },
                PwaConfig {
                    app_name: "Orders".into(),
                    app_scope: "/kiosk/orders/".into(),
                    ..Default::default()
                },
            ],
        };
        assert_eq!(set.resolve("/dashboard").app_name, "Root");
        assert_eq!(set.resolve("/kiosk").app_name, "Kiosk");
        assert_eq!(set.resolve("/kiosk/orders/1").app_name, "Orders");
        assert_eq!(set.resolve("/kioskette").app_name, "Root");
    }

    #[test]
    fn resolve_without_extra_apps_always_uses_root() {
        let set = PwaConfigSet {
            root: PwaConfig {
                app_name: "Only".into(),
                app_scope: "/portal/".into(),
                ..Default::default()
            },
            apps: vec![],
        };
        assert_eq!(set.resolve("/").app_name, "Only");
        assert_eq!(set.resolve("/portal/home").app_name, "Only");
        assert_eq!(set.resolve("/other").app_name, "Only");
    }

    #[test]
    fn resolve_scope_query_falls_back_to_root() {
        let set = PwaConfigSet {
            root: PwaConfig {
                app_name: "Root".into(),
                app_scope: "/".into(),
                ..Default::default()
            },
            apps: vec![PwaConfig {
                app_name: "Kiosk".into(),
                app_scope: "/kiosk/".into(),
                ..Default::default()
            }],
        };
        assert_eq!(set.resolve_scope_query(None).app_name, "Root");
        assert_eq!(set.resolve_scope_query(Some("")).app_name, "Root");
        assert_eq!(set.resolve_scope_query(Some("/kiosk/")).app_name, "Kiosk");
        assert_eq!(set.resolve_scope_query(Some("/missing")).app_name, "Root");
    }

    #[test]
    fn sanitize_drops_duplicate_scopes() {
        let set = PwaConfigSet {
            root: PwaConfig::default(),
            apps: vec![
                PwaConfig {
                    app_name: "First".into(),
                    app_scope: "/kiosk/".into(),
                    ..Default::default()
                },
                PwaConfig {
                    app_name: "Second".into(),
                    app_scope: "/kiosk".into(),
                    ..Default::default()
                },
            ],
        }
        .sanitize();
        assert_eq!(set.apps.len(), 1);
        assert_eq!(set.apps[0].app_name, "First");
    }

    #[test]
    fn flattened_section_deserializes_root_and_apps() {
        let set: PwaConfigSet = serde_json::from_str(
            r#"{
                "PWA_APP_NAME": "Lariv",
                "PWA_APP_SCOPE": "/",
                "apps": [
                    {"PWA_APP_NAME": "Kiosk", "PWA_APP_SCOPE": "/kiosk/"}
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(set.root.app_name, "Lariv");
        assert_eq!(set.apps.len(), 1);
        assert_eq!(set.apps[0].app_name, "Kiosk");
        assert_eq!(set.apps[0].scope(), "/kiosk");
    }

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
