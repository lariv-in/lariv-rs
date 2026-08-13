//! TOML configuration loading and the core [`AppConfig`] section.
//!
//! Plugins register config sections as tagged HList items. [`LoadFromToml`] deserializes
//! each section from the TOML document root or a named table via [`ConfigSection::KEY`].
//!
//! # Core configuration
//!
//! - [`AppConfigTag`] → [`AppConfig`] — `database_url`, `bind` / `uds` (document root)
//!
//! # Examples
//!
//! ```ignore
//! // Plugin config sections implement ConfigSection:
//! pub struct UsersConfigTag;
//! impl ConfigSection for UsersConfigTag {
//!     const KEY: Option<&'static str> = Some("users");
//! }
//! ```

use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;

use frunk::{HCons, HNil, hlist::HList};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::{
    app::App,
    capability::{CapStore, Capability},
    tag::Tagged,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::GetByTag,
    },
};

/// Capability tag for the aggregated config HList.
pub struct ConfigTag;

/// Tag for the core [`AppConfig`] section (document root).
pub struct AppConfigTag;

/// Marks a config tag with its TOML location.
pub trait ConfigSection {
    /// Table name, or `None` to deserialize from the document root.
    const KEY: Option<&'static str>;
}

impl ConfigSection for AppConfigTag {
    const KEY: Option<&'static str> = None;
}

/// Fill config values from a TOML document via [`Deserialize`].
pub trait LoadFromToml {
    fn load_from_toml(&mut self, root: &toml::Value) -> Result<(), toml::de::Error>;
}

impl LoadFromToml for HNil {
    fn load_from_toml(&mut self, _: &toml::Value) -> Result<(), toml::de::Error> {
        Ok(())
    }
}

impl<Tag, V, Tail> LoadFromToml for HCons<Tagged<Tag, V>, Tail>
where
    Tag: ConfigSection,
    V: DeserializeOwned,
    Tail: LoadFromToml,
{
    fn load_from_toml(&mut self, root: &toml::Value) -> Result<(), toml::de::Error> {
        match Tag::KEY {
            None => {
                self.head.value = V::deserialize(root.clone())?;
            }
            Some(key) => {
                if let Some(section) = root.get(key) {
                    self.head.value = V::deserialize(section.clone())?;
                }
            }
        }
        self.tail.load_from_toml(root)
    }
}

/// Mounted config capability: HList of tagged plugin/core config structs.
#[derive(Clone)]
pub struct ConfigCapability<Configs> {
    pub configs: Configs,
}

impl ConfigCapability<HNil> {
    pub fn new() -> Self {
        Self { configs: HNil }
    }
}

impl Default for ConfigCapability<HNil> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Configs> ConfigCapability<Configs> {
    pub fn prepend<Tag, V>(self, value: V) -> ConfigCapability<HCons<Tagged<Tag, V>, Configs>>
    where
        Configs: HList,
    {
        ConfigCapability {
            configs: HCons {
                head: Tagged::new(value),
                tail: self.configs,
            },
        }
    }

    pub fn get<Tag, Index>(&self) -> &<Configs as GetByTag<Tag, Index>>::Value
    where
        Configs: GetByTag<Tag, Index>,
    {
        self.configs.get_by_tag()
    }

    pub fn load_from_toml(mut self, root: &toml::Value) -> Result<Self, toml::de::Error>
    where
        Configs: LoadFromToml,
    {
        self.configs.load_from_toml(root)?;
        Ok(self)
    }
}

/// Builder-phase config capability (hooks usually empty; load is fallible prep).
pub type ConfigCap<Hooks, Items> = CapStore<ConfigTag, Hooks, Items>;

impl<Items> Capability for ConfigCap<HNil, Items> {
    type Value = ConfigCapability<Items>;
    type Output = Tagged<ConfigTag, ConfigCapability<Items>>;
    type Hooks = HNil;
    type Items = Items;

    fn mount(self) -> Self::Output {
        Tagged::new(ConfigCapability {
            configs: self.items,
        })
    }
}

/// Where the HTTP server should listen: TCP or a Unix domain socket.
///
/// Resolved from [`AppConfig`]: a non-empty `uds` overrides `bind`.
#[derive(Debug, Clone)]
pub enum BindTarget {
    /// TCP socket address (e.g. `0.0.0.0:3000`).
    Tcp(SocketAddr),
    /// Unix domain socket filesystem path (overrides TCP when set).
    Uds(PathBuf),
}

#[derive(Debug, Clone, Deserialize)]
/// Core application configuration loaded from the TOML document root.
///
/// Fields can be overridden by environment variables `DATABASE_URL`, `BIND`,
/// and `UDS` during [`App::load_config`](crate::app::App::load_config).
pub struct AppConfig {
    #[serde(default = "default_database_url")]
    pub database_url: String,
    #[serde(default = "default_bind")]
    pub bind: Option<String>,
    /// Unix domain socket path; when set, overrides [`Self::bind`].
    ///
    /// Accepts `uds` or Go-style `UDS` in TOML.
    #[serde(default, alias = "UDS")]
    pub uds: Option<String>,
}

fn default_database_url() -> String {
    "sqlite://data/lariv.db?mode=rwc".into()
}

fn default_bind() -> Option<String> {
    Some("127.0.0.1:3000".into())
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database_url: default_database_url(),
            bind: default_bind(),
            uds: None,
        }
    }
}

impl AppConfig {
    /// HTTP bind address, defaulting to `0.0.0.0:3000` when `bind` is unset.
    pub fn bind_addr(&self) -> &str {
        self.bind.as_deref().unwrap_or("0.0.0.0:3000")
    }

    /// Resolve listen target: non-empty `uds` wins over TCP `bind`.
    pub fn bind_target(&self) -> anyhow::Result<BindTarget> {
        if let Some(path) = self.uds.as_deref().filter(|s| !s.is_empty()) {
            return Ok(BindTarget::Uds(PathBuf::from(path)));
        }
        let addr: SocketAddr = self.bind_addr().parse()?;
        Ok(BindTarget::Tcp(addr))
    }
}

/// Error loading configuration or connecting to the database during prep.
#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Toml(toml::de::Error),
    Db(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Toml(e) => write!(f, "{e}"),
            Self::Db(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Toml(e) => Some(e),
            Self::Db(_) => None,
        }
    }
}

impl From<io::Error> for ConfigError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(value: toml::de::Error) -> Self {
        Self::Toml(value)
    }
}

type DefaultConfigItems = HCons<Tagged<AppConfigTag, AppConfig>, HNil>;

/// Add the config capability with default [`AppConfig`] (called by [`App::new_web_app`](crate::app::App::new_web_app)).
pub fn with_config<L, Proof>(app: App<L>) -> App<HCons<ConfigCap<HNil, DefaultConfigItems>, L>>
where
    L: HList + CapTagAbsent<ConfigTag, Proof>,
{
    app.add_capability(CapStore::with_items(
        ConfigCapability::new()
            .prepend::<AppConfigTag, _>(AppConfig::default())
            .configs,
    ))
}
