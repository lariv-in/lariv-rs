//! Inventory plugin compile smoke test.

#![recursion_limit = "512"]

use std::path::PathBuf;

use lariv_rs::app::App;
use lariv_rs::plugins::{contacts, crm, filesystem, inventory, users};

const MINIMAL_DB_TOML: &str = r#"database_url = "sqlite::memory:""#;

fn temp_config(name: &str, body: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lariv-inventory-compile-{name}-{}-{}.toml",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&path, body).expect("write temp config");
    path
}

#[test]
fn inventory_plugin_mounts() {
    std::thread::Builder::new()
        .name("inventory-plugin-mount".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async {
                let app = App::new_web_app();
                let app = users::install(app);
                let app = filesystem::install(app);
                let app = contacts::install(app);
                let app = crm::install(app);
                let app = inventory::install(app);
                let path = temp_config("db", MINIMAL_DB_TOML);
                let app = app.load_config(&path).await.expect("load_config");
                std::fs::remove_file(&path).ok();
                let _mounted = app.mount();
            });
        })
        .expect("spawn inventory mount thread")
        .join()
        .expect("inventory mount thread");
}
