//! Path-scoped PWA resource URLs (`?scope=`).

use std::sync::OnceLock;

use super::config::{PwaConfig, PwaConfigSet, normalize_scope};
use super::routes::{PwaFaviconRouteTag, PwaManifestRouteTag, PwaOfflineRouteTag};

static CONFIG_SET: OnceLock<PwaConfigSet> = OnceLock::new();
static SW_SCOPE_RESOLVER: OnceLock<fn(&str) -> Option<String>> = OnceLock::new();

/// Store the sanitized config set for path resolvers and handlers.
pub fn install_config_set(set: PwaConfigSet) -> PwaConfigSet {
    let set = set.sanitize();
    if CONFIG_SET.set(set.clone()).is_err() {
        tracing::warn!("p_pwa: config set already installed; keeping the first");
    }
    set
}

/// Installed multi-app config, if [`install_config_set`] has run.
pub fn config_set() -> Option<&'static PwaConfigSet> {
    CONFIG_SET.get()
}

/// Register the service-worker scope resolver used by the install script slot.
pub fn register_sw_scope_resolver(resolve: fn(&str) -> Option<String>) {
    if SW_SCOPE_RESOLVER.set(resolve).is_err() {
        tracing::warn!("p_pwa: service-worker scope resolver already registered");
    }
}

/// Service-worker registration scope for `request_path`.
///
/// `None` when multi-app resolvers are not installed (single-app default).
pub fn sw_scope_for_path(request_path: &str) -> Option<String> {
    SW_SCOPE_RESOLVER
        .get()
        .and_then(|resolve| resolve(request_path))
}

fn scope_query(scope: &str) -> String {
    // Scopes are normalized path prefixes (`/kiosk`); keep `/` readable in the query.
    let mut out = String::with_capacity(scope.len() * 3);
    for b in scope.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Manifest href for `cfg`. Root `/` scope stays unscoped for backward compatibility.
pub fn manifest_href(cfg: &PwaConfig) -> String {
    let scope = cfg.scope();
    if scope == "/" {
        PwaManifestRouteTag::PATH.to_owned()
    } else {
        format!(
            "{}?scope={}",
            PwaManifestRouteTag::PATH,
            scope_query(&scope)
        )
    }
}

/// Favicon href for `cfg`, or `None` when `faviconPath` is empty.
pub fn favicon_href(cfg: &PwaConfig) -> Option<String> {
    if cfg.favicon_path.is_empty() {
        return None;
    }
    let scope = cfg.scope();
    if scope == "/" {
        Some(PwaFaviconRouteTag::PATH.to_owned())
    } else {
        Some(format!(
            "{}?scope={}",
            PwaFaviconRouteTag::PATH,
            scope_query(&scope)
        ))
    }
}

/// Offline fallback URL for a service-worker registration scope.
pub fn offline_href_for_sw_scope(sw_scope: &str) -> String {
    let scope = normalize_scope(sw_scope);
    if scope == "/" {
        PwaOfflineRouteTag::PATH.to_owned()
    } else {
        format!(
            "{}?scope={}",
            PwaOfflineRouteTag::PATH,
            scope_query(&scope)
        )
    }
}

/// Append `?scope=` to `/static/pwa` icon/screenshot URLs when the app has its own `staticDir`.
pub fn rewrite_static_src(src: &str, cfg: &PwaConfig) -> String {
    if cfg.static_dir.is_empty() {
        return src.to_owned();
    }
    let scope = cfg.scope();
    if scope == "/" {
        return src.to_owned();
    }
    if !src.starts_with("/static/pwa") {
        return src.to_owned();
    }
    if src.contains('?') {
        return src.to_owned();
    }
    format!("{src}?scope={}", scope_query(&scope))
}

/// Path resolver: manifest href for the matched app.
pub fn resolve_manifest_path(request_path: &str) -> Option<String> {
    let set = config_set()?;
    Some(manifest_href(set.resolve(request_path)))
}

/// Path resolver: favicon href for the matched app.
pub fn resolve_favicon_path(request_path: &str) -> Option<String> {
    let set = config_set()?;
    favicon_href(set.resolve(request_path))
}

/// Path resolver: Apple head HTML for the matched app.
pub fn resolve_apple_head(request_path: &str) -> Option<String> {
    let set = config_set()?;
    let html = super::apple::apple_head(set.resolve(request_path)).into_string();
    (!html.is_empty()).then_some(html)
}

/// Path resolver: document title for the matched app.
pub fn resolve_document_title(request_path: &str) -> Option<String> {
    let set = config_set()?;
    let name = set.resolve(request_path).app_name.clone();
    (!name.is_empty()).then_some(name)
}

/// Path resolver: service-worker registration scope for the matched app.
///
/// Non-root scopes use a trailing slash so the worker does not claim sibling paths.
pub fn resolve_sw_scope(request_path: &str) -> Option<String> {
    let set = config_set()?;
    let scope = set.resolve(request_path).scope();
    if scope == "/" {
        Some("/".into())
    } else {
        Some(format!("{scope}/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_scope_urls_stay_unscoped() {
        let cfg = PwaConfig {
            app_scope: "/".into(),
            favicon_path: "icon.ico".into(),
            ..Default::default()
        };
        assert_eq!(manifest_href(&cfg), "/app.webmanifest");
        assert_eq!(favicon_href(&cfg).as_deref(), Some("/favicon.ico"));
        assert_eq!(offline_href_for_sw_scope("/"), "/offline");
    }

    #[test]
    fn nested_scope_urls_carry_query() {
        let cfg = PwaConfig {
            app_scope: "/kiosk/".into(),
            favicon_path: "icon.ico".into(),
            static_dir: "kiosk-pwa".into(),
            ..Default::default()
        };
        assert_eq!(manifest_href(&cfg), "/app.webmanifest?scope=/kiosk");
        assert_eq!(
            favicon_href(&cfg).as_deref(),
            Some("/favicon.ico?scope=/kiosk")
        );
        assert_eq!(
            offline_href_for_sw_scope("/kiosk/"),
            "/offline?scope=/kiosk"
        );
        assert_eq!(
            rewrite_static_src("/static/pwa/icon-192.png", &cfg),
            "/static/pwa/icon-192.png?scope=/kiosk"
        );
        assert_eq!(
            rewrite_static_src("https://cdn.example/icon.png", &cfg),
            "https://cdn.example/icon.png"
        );
    }
}
