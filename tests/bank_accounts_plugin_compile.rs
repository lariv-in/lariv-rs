//! Bank accounts plugin compile smoke test.

#![recursion_limit = "512"]

use std::path::PathBuf;

use lariv_rs::app::App;
use lariv_rs::plugins::{bank_accounts, users};

const MINIMAL_DB_TOML: &str = r#"database_url = "sqlite::memory:""#;

fn temp_config(name: &str, body: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lariv-bank-accounts-compile-{name}-{}-{}.toml",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&path, body).expect("write temp config");
    path
}

#[tokio::test]
async fn bank_accounts_plugin_mounts() {
    let app = App::new_web_app();
    let app = users::install(app);
    let app = bank_accounts::install(app);
    let path = temp_config("db", MINIMAL_DB_TOML);
    let app = app.load_config(&path).await.expect("load_config");
    std::fs::remove_file(&path).ok();
    let _mounted = app.mount();
}
