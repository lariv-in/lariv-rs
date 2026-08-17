//! Per-plugin compile smoke tests.
//!
//! Each test installs one plugin (plus documented minimum dependencies) on
//! [`App::new_web_app`], runs [`App::load_config`] when DB/state hooks are
//! required, then [`App::mount`] to force the full deferred-hook type chain to
//! resolve. Failure to compile means a plugin's install wiring is broken.

#![recursion_limit = "512"]

use std::path::PathBuf;

use lariv_rs::app::App;
use lariv_rs::plugins::{
    blog, dashboard, filesystem, import, llm_assistant, otp, pwa, signup, users, website,
};

const MINIMAL_DB_TOML: &str = r#"database_url = "sqlite::memory:""#;

fn temp_config(name: &str, body: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lariv-plugin-compile-{name}-{}-{}.toml",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&path, body).expect("write temp config");
    path
}

/// Load sqlite memory config, mount, and drop the temp file.
macro_rules! mount_with_db {
    ($app:expr) => {{
        let path = temp_config("db", MINIMAL_DB_TOML);
        let app = $app.load_config(&path).await.expect("load_config");
        std::fs::remove_file(&path).ok();
        app.mount()
    }};
}

// --- plugins with no DB/state hooks ---

#[tokio::test]
async fn dashboard_plugin_mounts() {
    let app = App::new_web_app();
    let app = dashboard::install(app);
    let _mounted = app.mount();
}

// --- plugins that need config + DB (or config-only state) ---

#[tokio::test]
async fn pwa_plugin_mounts() {
    let app = App::new_web_app();
    let app = pwa::install(app);
    let _mounted = mount_with_db!(app);
}

#[tokio::test]
async fn users_plugin_mounts() {
    let app = App::new_web_app();
    let app = users::install(app);
    let _mounted = mount_with_db!(app);
}

#[tokio::test]
async fn signup_plugin_mounts() {
    let app = App::new_web_app();
    let app = users::install(app);
    let app = signup::install(app);
    let _mounted = mount_with_db!(app);
}

#[tokio::test]
async fn otp_plugin_mounts() {
    let app = App::new_web_app();
    let app = users::install(app);
    let app = otp::install(app);
    let _mounted = mount_with_db!(app);
}

#[tokio::test]
async fn blog_plugin_mounts() {
    let app = App::new_web_app();
    let app = blog::install(app);
    let _mounted = mount_with_db!(app);
}

#[tokio::test]
async fn filesystem_plugin_mounts() {
    let app = App::new_web_app();
    let app = filesystem::install(app);
    let _mounted = mount_with_db!(app);
}

/// Website state attaches a filestore and requires `[filesystem]` in the config HList.
#[tokio::test]
async fn website_plugin_mounts() {
    let app = App::new_web_app();
    let app = filesystem::install(app);
    let app = website::install(app);
    let _mounted = mount_with_db!(app);
}

#[tokio::test]
async fn llm_assistant_plugin_mounts() {
    let app = App::new_web_app();
    let app = llm_assistant::install(app);
    let _mounted = mount_with_db!(app);
}

#[tokio::test]
async fn import_plugin_mounts() {
    let app = App::new_web_app();
    let app = users::install(app);
    let app = import::install(app);
    let _mounted = mount_with_db!(app);
}

// --- full production stack (mirrors `src/bin/lariv.rs`) ---

#[test]
fn all_plugins_mounts() {
    std::thread::Builder::new()
        .name("all-plugins-mount".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async {
                let app = App::new_web_app();
                let app = users::install(app);
                let app = otp::install(app);
                let app = blog::install(app);
                let app = filesystem::install(app);
                let app = website::install(app);
                let app = llm_assistant::install(app);
                let app = import::install(app);
                let app = pwa::install(app);
                let app = dashboard::install(app);
                let _mounted = mount_with_db!(app);
            });
        })
        .expect("spawn all-plugins-mount thread")
        .join()
        .expect("all-plugins-mount thread");
}
