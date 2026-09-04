//! Integration tests for website GrapesJS components and page rendering.

#![recursion_limit = "512"]

use std::sync::Arc;

use lariv_rs::grapesjs::{GrapesJsCapability, GrapesJsRegistrar};
use lariv_rs::plugins::website::{component_catalog, grapesjs::Hook, publish};

fn catalog_from_hook() -> Arc<GrapesJsCapability> {
    component_catalog::build_website_catalog()
}

#[test]
fn grapesjs_hook_registers_all_landing_blocks() {
    let gjs = catalog_from_hook();
    let block_ids: Vec<_> = gjs.blocks().iter().map(|(id, _)| id.as_str()).collect();
    for id in [
        "p_website.section-header",
        "p_website.feature-card",
        "p_website.contact-detail",
        "p_website.expand-section",
        "p_website.row-3",
        "p_website.section",
        "p_website.hero",
        "p_website.navbar",
        "p_website.cta",
        "p_website.video",
    ] {
        assert!(block_ids.contains(&id), "missing block {id}");
    }
}

#[test]
fn hero_component_preserves_media_subcomponent_registration() {
    let mut gjs = GrapesJsCapability::new();
    Hook.register_grapesjs(&mut gjs);
    assert!(component_catalog::component_has_init(
        &gjs,
        "p_website.hero"
    ));
    assert!(
        gjs.components()
            .iter()
            .any(|(id, _)| id == "p_website.hero-media"),
        "hero-media subcomponent must be registered"
    );
}

#[test]
fn published_landing_page_renders_fixture_markup() {
    let gjs = catalog_from_hook();
    let html = component_catalog::publish_landing_fixture(&gjs);
    assert!(html.contains("<!DOCTYPE html>") || html.contains("<html"));
    assert!(html.contains("Manufacturing Services"));
    assert!(html.contains("Tight-Tolerance Capability"));
    assert!(html.contains("tel:+918210098176"));
    assert!(html.contains("--color-accent: #f4ca4a"));
    assert!(html.contains("Rajdhani"));
}

#[test]
fn navbar_logo_property_survives_publish_fix() {
    let html = r#"<nav data-gjs-type="p_website.navbar" data-logo-src="https://example.com/logo.svg" data-logo-alt="Logo" class="gjs-navbar navbar">
<a href="/"><img alt="Logo" class="gjs-navbar-logo h-8" src="data:image/svg+xml;base64,PHN2Zy"></a>
</nav>"#;
    let fixed = publish::fix_navbar_logos(html);
    assert!(fixed.contains("src=\"https://example.com/logo.svg\""));
}

#[test]
fn layout_row_block_uses_feature_grid() {
    let gjs = catalog_from_hook();
    let html = component_catalog::block_html_for(&gjs, "p_website.row-3").expect("row-3");
    assert!(html.contains("feature-grid"));
}

#[test]
fn section_component_exposes_background_trait() {
    let gjs = catalog_from_hook();
    let names = component_catalog::component_trait_names(&gjs, "p_website.section");
    assert!(names.iter().any(|n| n == "data-section-bg"));
}

#[test]
fn contact_detail_component_exposes_href_trait() {
    let gjs = catalog_from_hook();
    let names = component_catalog::component_trait_names(&gjs, "p_website.contact-detail");
    assert!(names.iter().any(|n| n == "href"));
}

#[test]
fn video_component_exposes_orientation_sources() {
    let gjs = catalog_from_hook();
    let names = component_catalog::component_trait_names(&gjs, "p_website.video");
    assert!(names.iter().any(|n| n == "data-src-landscape"));
    assert!(names.iter().any(|n| n == "data-src-portrait"));
    assert!(component_catalog::component_has_script(
        &gjs,
        "p_website.video"
    ));
}
