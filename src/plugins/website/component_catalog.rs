//! GrapesJS website component catalog validation and publish fixtures.

use std::collections::HashMap;
use std::sync::Arc;

use crate::grapesjs::{GrapesJsCapability, GrapesJsRegistrar};
use crate::plugins::website::{grapesjs::Hook, publish};

/// Build the website plugin GrapesJS catalog (same registry as production).
pub fn build_website_catalog() -> Arc<GrapesJsCapability> {
    let mut gjs = GrapesJsCapability::new();
    Hook.register_grapesjs(&mut gjs);
    Arc::new(gjs)
}

/// Landing-page component types used for kdstagore-style sites.
pub const LANDING_COMPONENT_TYPES: &[&str] = &[
    "p_website.section",
    "p_website.row-2",
    "p_website.row-3",
    "p_website.card",
    "p_website.section-header",
    "p_website.feature-card",
    "p_website.contact-detail",
    "p_website.expand-section",
    "p_website.hero",
    "p_website.hero-media",
    "p_website.hero-inner",
    "p_website.navbar",
    "p_website.cta",
];

/// Expected trait attribute names per component type (subset we rely on in the builder).
pub fn expected_traits() -> HashMap<&'static str, Vec<&'static str>> {
    HashMap::from([
        ("p_website.section", vec!["data-section-bg"]),
        ("p_website.section-header", vec!["data-align", "data-header-style"]),
        ("p_website.hero", vec!["data-show-media", "data-show-button"]),
        ("p_website.cta", vec!["data-show-button"]),
        ("p_website.navbar", vec!["data-logo-src", "data-nav-links", "data-variant"]),
        ("p_website.contact-detail", vec!["href"]),
        ("p_website.heading", vec!["data-level"]),
    ])
}

pub fn block_html_for(gjs: &GrapesJsCapability, block_id: &str) -> Option<String> {
    gjs.blocks()
        .iter()
        .find(|(id, _)| id == block_id)
        .and_then(|(_, block)| block.content.as_str().map(str::to_string))
}

pub fn component_trait_names(gjs: &GrapesJsCapability, type_id: &str) -> Vec<String> {
    let Some((_, component)) = gjs.components().iter().find(|(id, _)| id == type_id) else {
        return Vec::new();
    };
    let Some(model) = component.model.as_ref() else {
        return Vec::new();
    };
    let Some(defaults) = model.get("defaults") else {
        return Vec::new();
    };
    let Some(traits) = defaults.get("traits").and_then(|t| t.as_array()) else {
        return Vec::new();
    };
    traits
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(String::from))
        .collect()
}

pub fn component_has_init(gjs: &GrapesJsCapability, type_id: &str) -> bool {
    gjs.components()
        .iter()
        .find(|(id, _)| id == type_id)
        .and_then(|(_, c)| c.model.as_ref())
        .and_then(|m| m.get("init"))
        .is_some_and(|v| v.as_str().is_some_and(|s| !s.is_empty()))
}

/// Minimal landing page HTML using typed components (mirrors manual builder composition).
pub fn landing_page_fixture() -> &'static str {
    include_str!("assets/fixtures/landing_page.html")
}

pub fn publish_landing_fixture(gjs: &GrapesJsCapability) -> String {
    publish::finalize_published_html(landing_page_fixture(), "", "p_website.kds", gjs.theme("p_website.kds"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn landing_component_types_are_registered() {
        let gjs = build_website_catalog();
        let registered: Vec<_> = gjs.components().iter().map(|(id, _)| id.as_str()).collect();
        for type_id in LANDING_COMPONENT_TYPES {
            assert!(
                registered.contains(type_id),
                "missing component type {type_id}"
            );
        }
    }

    #[test]
    fn landing_blocks_contain_matching_data_gjs_type() {
        let gjs = build_website_catalog();
        let block_ids = [
            "p_website.section",
            "p_website.row-3",
            "p_website.section-header",
            "p_website.feature-card",
            "p_website.contact-detail",
            "p_website.expand-section",
            "p_website.hero",
            "p_website.navbar",
            "p_website.cta",
        ];
        for block_id in block_ids {
            let html = block_html_for(&gjs, block_id).unwrap_or_else(|| panic!("block {block_id}"));
            assert!(
                html.contains(&format!("data-gjs-type=\"{block_id}\"")),
                "block {block_id} html missing data-gjs-type"
            );
        }
    }

    #[test]
    fn component_traits_match_expectations() {
        let gjs = build_website_catalog();
        for (type_id, expected) in expected_traits() {
            let names = component_trait_names(&gjs, type_id);
            for trait_name in expected {
                assert!(
                    names.iter().any(|n| n == trait_name),
                    "component {type_id} missing trait {trait_name}, got {names:?}"
                );
            }
        }
    }

    #[test]
    fn interactive_components_define_init_scripts() {
        let gjs = build_website_catalog();
        for type_id in [
            "p_website.hero",
            "p_website.cta",
            "p_website.section",
            "p_website.section-header",
            "p_website.navbar",
        ] {
            assert!(
                component_has_init(&gjs, type_id),
                "component {type_id} should define an init script"
            );
        }
    }

    #[test]
    fn landing_fixture_exposes_component_property_attributes() {
        let html = landing_page_fixture();
        let attrs = [
            ("p_website.hero", "data-show-media=\"true\""),
            ("p_website.hero", "data-show-button=\"false\""),
            ("p_website.navbar", "data-variant=\"kds\""),
            ("p_website.section", "data-section-bg=\"base\""),
            ("p_website.section-header", "data-align=\"center\""),
            ("p_website.section-header", "data-header-style=\"default\""),
            ("p_website.cta", "data-show-button=\"false\""),
            ("p_website.contact-detail", "href=\"tel:+918210098176\""),
        ];
        for (type_id, attr) in attrs {
            assert!(
                html.contains(&format!("data-gjs-type=\"{type_id}\"")) && html.contains(attr),
                "fixture missing {type_id} with {attr}"
            );
        }
    }

    #[test]
    fn landing_fixture_publishes_with_kds_theme() {
        let gjs = build_website_catalog();
        let html = publish_landing_fixture(&gjs);
        assert!(html.contains("<html"), "expected wrapped html document");
        assert!(html.contains("data-lariv-theme=\"p_website.kds\""));
        assert!(html.contains("--color-accent: #f4ca4a"));
        assert!(html.contains("Rajdhani"));
        assert!(html.contains("Manufacturing Services"));
        assert!(html.contains("data-gjs-type=\"p_website.feature-card\""));
        assert!(html.contains("data-gjs-type=\"p_website.contact-detail\""));
        assert!(html.contains("data-variant=\"kds\""));
        assert!(html.contains("data-show-media=\"true\""));
        assert!(html.contains("class=\"gjs-feature-card feature\""));
        assert!(html.contains("class=\"gjs-cta-box cta-box\""));
    }

    #[test]
    fn navbar_logo_fix_runs_on_fixture() {
        let html = r#"<nav data-gjs-type="p_website.navbar" data-logo-src="https://example.com/logo.svg" data-logo-alt="Logo" class="gjs-navbar navbar">
<a href="/"><img alt="Logo" class="gjs-navbar-logo h-8" src="data:image/svg+xml;base64,PHN2Zy"></a>
</nav>"#;
        let fixed = publish::fix_navbar_logos(html);
        assert!(fixed.contains("src=\"https://example.com/logo.svg\""));
        assert!(!fixed.contains("data:image/svg+xml"));
    }

    #[test]
    fn row_block_uses_feature_grid() {
        let gjs = build_website_catalog();
        let html = block_html_for(&gjs, "p_website.row-3").expect("row-3 block");
        assert!(html.contains("feature-grid"));
    }
}
