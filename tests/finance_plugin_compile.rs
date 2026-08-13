//! Finance plugin compile smoke tests.

#![recursion_limit = "512"]

use std::path::PathBuf;

use lariv_rs::app::App;
use lariv_rs::plugins::{
    customer, finance_accounts, finance_creditnotes, finance_customer, finance_fiscal_year,
    finance_indian, finance_invoices, finance_products, finance_taxes, users,
};

const MINIMAL_DB_TOML: &str = r#"database_url = "sqlite::memory:""#;

fn temp_config(name: &str, body: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lariv-finance-compile-{name}-{}-{}.toml",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&path, body).expect("write temp config");
    path
}

macro_rules! mount_with_db {
    ($app:expr) => {{
        let path = temp_config("db", MINIMAL_DB_TOML);
        let app = $app.load_config(&path).await.expect("load_config");
        std::fs::remove_file(&path).ok();
        app.mount()
    }};
}

#[tokio::test]
async fn finance_accounts_plugin_mounts() {
    let app = App::new_web_app();
    let app = users::install(app);
    let app = finance_accounts::install(app);
    let _mounted = mount_with_db!(app);
}

#[test]
fn finance_stack_mounts() {
    std::thread::Builder::new()
        .name("finance-stack-mount".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async {
                let app = App::new_web_app();
                let app = users::install(app);
                let app = finance_accounts::install(app);
                let app = customer::install(app);
                let app = finance_customer::install(app);
                let app = finance_creditnotes::install(app);
                let app = finance_fiscal_year::install(app);
                let app = finance_taxes::install(app);
                let app = finance_products::install(app);
                let app = finance_invoices::install(app);
                let app = finance_indian::install(app);
                let _mounted = mount_with_db!(app);
            });
        })
        .expect("spawn finance-stack-mount thread")
        .join()
        .expect("finance-stack-mount thread");
}
