//! iOS Safari PWA `<head>` tags — Apple still requires these instead of the web manifest.

use maud::{Markup, html};

use super::config::PwaConfig;

fn attr(s: &str) -> Option<&str> {
    (!s.is_empty()).then_some(s)
}

/// Apple / iOS standalone-app tags from `[pwa]` config.
///
/// Safari ignores most web-manifest members, so home-screen install needs:
/// `apple-mobile-web-app-capable`, `apple-mobile-web-app-title`,
/// `apple-mobile-web-app-status-bar-style`, `theme-color`,
/// `apple-touch-icon`, and `apple-touch-startup-image`.
pub fn apple_head(cfg: &PwaConfig) -> Markup {
    let title = attr(cfg.short_name());
    let theme = attr(&cfg.app_theme_color);
    let status_bar = if cfg.app_status_bar_color.is_empty() {
        "default"
    } else {
        cfg.app_status_bar_color.as_str()
    };
    let icons: Vec<_> = if cfg.app_icons_apple.iter().any(|i| !i.src.is_empty()) {
        cfg.app_icons_apple
            .iter()
            .filter(|i| !i.src.is_empty())
            .map(|i| (i.src.as_str(), i.sizes.as_str(), i.type_.as_str()))
            .collect()
    } else {
        cfg.app_icons
            .iter()
            .filter(|i| !i.src.is_empty())
            .map(|i| (i.src.as_str(), i.sizes.as_str(), i.type_.as_str()))
            .collect()
    };

    html! {
        meta name="mobile-web-app-capable" content="yes";
        meta name="apple-mobile-web-app-capable" content="yes";
        meta name="apple-mobile-web-app-status-bar-style" content=(status_bar);
        @if let Some(title) = title {
            meta name="apple-mobile-web-app-title" content=(title);
        }
        @if let Some(theme) = theme {
            meta name="theme-color" content=(theme);
        }
        @for (src, sizes, type_) in icons {
            link rel="apple-touch-icon" href=(src) sizes=[attr(sizes)] type=[attr(type_)];
        }
        @for splash in cfg.app_splash_screen.iter().filter(|s| !s.src.is_empty()) {
            link rel="apple-touch-startup-image"
                href=(splash.src)
                media=[attr(&splash.media)]
                sizes=[attr(&splash.sizes)]
                type=[attr(&splash.type_)];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::pwa::config::{PwaAppleIconConfig, PwaIconConfig, PwaSplashScreenConfig};

    fn markup_str(m: Markup) -> String {
        m.into_string()
    }

    #[test]
    fn apple_head_emits_ios_standalone_tags() {
        let html = markup_str(apple_head(&PwaConfig {
            app_name: "Lariv".into(),
            app_theme_color: "#0f172a".into(),
            app_status_bar_color: "black-translucent".into(),
            app_icons_apple: vec![PwaAppleIconConfig {
                src: "/static/pwa/apple-touch-icon.png".into(),
                sizes: "180x180".into(),
                type_: "image/png".into(),
            }],
            app_splash_screen: vec![PwaSplashScreenConfig {
                src: "/static/pwa/splash.png".into(),
                media: "(device-width: 430px) and (device-height: 932px)".into(),
                ..Default::default()
            }],
            ..Default::default()
        }));
        assert!(html.contains(r#"name="mobile-web-app-capable""#));
        assert!(html.contains(r#"content="yes""#));
        assert!(html.contains(r#"name="apple-mobile-web-app-capable""#));
        assert!(html.contains(r#"name="apple-mobile-web-app-title""#));
        assert!(html.contains(r#"content="Lariv""#));
        assert!(html.contains(r#"name="apple-mobile-web-app-status-bar-style""#));
        assert!(html.contains(r#"content="black-translucent""#));
        assert!(html.contains(r#"name="theme-color""#));
        assert!(html.contains("#0f172a"));
        assert!(html.contains(r#"rel="apple-touch-icon""#));
        assert!(html.contains("/static/pwa/apple-touch-icon.png"));
        assert!(html.contains(r#"sizes="180x180""#));
        assert!(html.contains(r#"rel="apple-touch-startup-image""#));
        assert!(html.contains("/static/pwa/splash.png"));
        assert!(html.contains("device-width: 430px"));
    }

    #[test]
    fn apple_head_falls_back_to_manifest_icons_and_default_status_bar() {
        let html = markup_str(apple_head(&PwaConfig {
            app_icons: vec![PwaIconConfig {
                src: "/static/pwa/icon-192.png".into(),
                sizes: "192x192".into(),
                ..Default::default()
            }],
            ..Default::default()
        }));
        assert!(html.contains(r#"content="default""#));
        assert!(!html.contains(r#"name="apple-mobile-web-app-title""#));
        assert!(!html.contains(r#"name="theme-color""#));
        assert!(html.contains(r#"rel="apple-touch-icon""#));
        assert!(html.contains("/static/pwa/icon-192.png"));
        assert!(!html.contains(r#"type=""#));
    }

    #[test]
    fn apple_head_prefers_short_name_for_home_screen_title() {
        let html = markup_str(apple_head(&PwaConfig {
            app_name: "Lariv Progressive Web App".into(),
            app_short_name: "Lariv".into(),
            ..Default::default()
        }));
        assert!(html.contains(r#"name="apple-mobile-web-app-title""#));
        assert!(html.contains(r#"content="Lariv""#));
        assert!(!html.contains("Lariv Progressive Web App"));
    }
}
