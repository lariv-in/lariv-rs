//! Invoice log plugin compile smoke test.

#![recursion_limit = "512"]

use std::path::PathBuf;

use lariv_rs::app::App;
use lariv_rs::plugins::{
    finance_accounts, finance_customers, finance_invoices, finance_invoices_log, logging, users,
};

#[cfg(feature = "plugin-llm-assistant")]
use lariv_rs::plugins::{filesystem, llm_assistant};

const MINIMAL_DB_TOML: &str = r#"database_url = "sqlite::memory:""#;

fn temp_config(name: &str, body: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lariv-finance-invoices-log-compile-{name}-{}-{}.toml",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&path, body).expect("write temp config");
    path
}

#[test]
fn finance_invoices_log_plugin_mounts() {
    std::thread::Builder::new()
        .name("finance-invoices-log-mount".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async {
                let app = App::new_web_app();
                let app = users::install(app);
                #[cfg(feature = "plugin-llm-assistant")]
                let app = filesystem::install(app);
                #[cfg(feature = "plugin-llm-assistant")]
                let app = llm_assistant::install(app);
                let app = finance_accounts::install(app);
                let app = finance_customers::install(app);
                let app = finance_invoices::install(app);
                let app = logging::install(app);
                let app = finance_invoices_log::install(app);
                let path = temp_config("db", MINIMAL_DB_TOML);
                let app = app.load_config(&path).await.expect("load_config");
                std::fs::remove_file(&path).ok();
                let _mounted = app.mount();
            });
        })
        .expect("spawn finance-invoices-log-mount thread")
        .join()
        .expect("finance-invoices-log-mount thread");
}
