//! Plugin configuration from TOML.
//!
//! # Config sections (`config.rs`)
//!
//! Plugins declare a config struct and tag, implement [`ConfigSection`](crate::config::ConfigSection),
//! and register the section in [`define_plugin_install!`](crate::plugin_install::define_plugin_install):
//!
//! ```ignore
//! use serde::Deserialize;
//! use lariv_rs::config::ConfigSection;
//!
//! pub struct MyPluginConfigTag;
//!
//! impl ConfigSection for MyPluginConfigTag {
//!     const KEY: Option<&'static str> = Some("my_plugin");
//! }
//!
//! #[derive(Debug, Clone, Deserialize)]
//! pub struct MyPluginConfig {
//!     #[serde(default = "default_api_key")]
//!     pub api_key: String,
//!     #[serde(default = "default_max_retries")]
//!     pub max_retries: u32,
//! }
//!
//! fn default_api_key() -> String { String::new() }
//! fn default_max_retries() -> u32 { 3 }
//!
//! impl Default for MyPluginConfig {
//!     fn default() -> Self {
//!         Self { api_key: default_api_key(), max_retries: default_max_retries() }
//!     }
//! }
//! ```
//!
//! Register in install steps:
//!
//! ```ignore
//! define_plugin_install! {
//!     plugin: MyPluginTag;
//!     steps: [
//!         config(MyPluginConfigTag, MyPluginConfig),
//!         // …
//!     ]
//! }
//! ```
//!
//! # TOML file
//!
//! ```toml
//! database_url = "sqlite://data/lariv.db?mode=rwc"
//! bind = "127.0.0.1:3000"
//!
//! [my_plugin]
//! api_key = "secret"
//! max_retries = 5
//! ```
//!
//! Core settings (`database_url`, `bind`) live at the document root under [`AppConfig`](crate::config::AppConfig).
//! Plugin sections use their `ConfigSection::KEY` table name.
//!
//! # Reading config at runtime
//!
//! During [`AttachState`](crate::hooks::AttachState), read config from the mounted
//! [`ConfigCapability`](crate::config::ConfigCapability):
//!
//! ```ignore
//! let cfg = app.get_capability::<ConfigTag, _>()
//!     .items
//!     .get::<MyPluginConfigTag, _>()
//!     .clone();
//! ```
//!
//! # Environment overrides
//!
//! [`App::load_config`](crate::app::App::load_config) also reads `DATABASE_URL` and `BIND`
//! environment variables, overriding TOML values.
