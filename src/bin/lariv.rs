#![recursion_limit = "512"]

use lariv_rs::app::App;
use lariv_rs::plugins::blog;
use lariv_rs::plugins::dashboard;
use lariv_rs::plugins::filesystem;
use lariv_rs::plugins::llm_assistant;
use lariv_rs::plugins::no_signup;
use lariv_rs::plugins::otp;
use lariv_rs::plugins::pwa;
use lariv_rs::plugins::users;
use lariv_rs::plugins::website;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let app = App::new_web_app();
    let app = users::install(app);
    let app = otp::install(app);
    let app = no_signup::install(app);
    let app = blog::install(app);
    let app = filesystem::install(app);
    let app = website::install(app);
    let app = llm_assistant::install(app);
    let app = pwa::install(app);
    let app = dashboard::install(app);

    let app = app.load_config("config.toml").await?;
    let app = app.mount();
    app.run().await?;
    Ok(())
}
