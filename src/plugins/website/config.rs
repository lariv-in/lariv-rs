//! Website plugin configuration (`[website]` in TOML), aligned with Go `p_website.WebsiteConfig`.

use serde::{Deserialize, Serialize};

use crate::config::ConfigSection;

/// Config HList tag for [`WebsiteConfig`].
pub struct WebsiteConfigTag;

impl ConfigSection for WebsiteConfigTag {
    const KEY: Option<&'static str> = Some("website");
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WebsiteConfig {
    /// VNode directory for "Create new HTML file" (e.g. `website/pages`).
    #[serde(default, rename = "newPageRootDir")]
    pub new_page_root_dir: String,
    /// VNode directory for GrapesJS uploads. Empty → `{newPageRootDir}/assets` or `assets`.
    #[serde(default, rename = "assetsDir")]
    pub assets_dir: String,
}

impl WebsiteConfig {
    pub fn resolved_assets_dir(&self) -> String {
        let assets = self.assets_dir.trim();
        if !assets.is_empty() {
            return assets.to_string();
        }
        let root = self.new_page_root_dir.trim().trim_matches('/');
        if root.is_empty() {
            "assets".into()
        } else {
            format!("{root}/assets")
        }
    }

    pub fn new_page_root_segments(&self) -> Vec<String> {
        self.new_page_root_dir
            .trim()
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    pub fn assets_dir_segments(&self) -> Vec<String> {
        self.resolved_assets_dir()
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }
}
