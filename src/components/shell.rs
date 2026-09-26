//! Document shells wrapping DaisyUI / HTMX / Alpine chrome.

mod auth;
mod base;
mod scaffold;
mod simple;
mod topbar;
mod vendor;

pub use auth::{ShellAuth, shell_auth};
pub use base::{
    ShellBase, app_manifest_path, apple_pwa_head, favicon_path, pwa_head_html,
    set_app_manifest_path, set_apple_pwa_head, set_favicon_path, shell_base,
};
pub use scaffold::{ShellScaffold, shell_scaffold};
pub use simple::{ShellSimple, shell_simple};
pub use topbar::{ShellTopbar, shell_topbar};
pub use vendor::{
    apexcharts_script, bundle_css_href, bundle_js_href, mount_vendor_bundles, vendor_head,
};
