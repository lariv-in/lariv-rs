//! Application lifecycle: builder phase, mount, and runtime CLI/HTTP.
//!
//! New to Lariv? Start with the [`quickstart guide`](crate::docs::quickstart).
//!
//! Lariv apps are built in two phases:
//!
//! 1. **Builder** ([`App`]) — plugins install capabilities and deferred hooks onto an HList.
//! 2. **Mounted** ([`MountedApp`]) — hooks are resolved, capabilities folded to [`Tagged`](crate::tag::Tagged)
//!    values, and the app is ready for migrations, seeds, CLI, or HTTP serving.
//!
//! # Getting started
//!
//! ```ignore
//! use lariv_rs::app::App;
//!
//! let app = App::new_web_app();
//! // Install plugins (each adds hooks + config sections):
//! // let app = lariv_rs::plugins::users::install(app);
//! let app = app.load_config("config.toml").await?;
//! let mounted = app.mount();
//! mounted.run().await?; // defaults to `serve`
//! ```
//!
//! # Lifecycle methods
//!
//! | Phase | Method | Purpose |
//! |-------|--------|---------|
//! | Builder | [`App::new_web_app`] | Empty app with core capabilities (HTTP, templates, CLI, …) |
//! | Builder | `plugin::install(app)` | Register plugin hooks (via [`define_plugin_install!`](crate::plugin_install::define_plugin_install)) |
//! | Builder | [`App::load_config`] | Load TOML, connect DB, run state-attachment hooks |
//! | Builder | [`App::mount`] | Fold each capability's hooks into its mounted value |
//! | Mounted | [`MountedApp::run_migrations`] | Apply SeaORM migrations from all plugins |
//! | Mounted | [`MountedApp::run_seeds`] | Run startup seed hooks |
//! | Mounted | [`MountedApp::run_serve_startups`] | Run serve-only startup hooks (e.g. IMAP listener) |
//! | Mounted | [`MountedApp::serve`] | Start Axum HTTP server |
//! | Mounted | [`MountedApp::run`] | Parse CLI and dispatch command (default: serve) |

use std::path::Path;

use frunk::{HCons, HNil, hlist, hlist::HList};
use sea_orm::DbErr;
use tower::{Layer, make::Shared};
use tower_http::normalize_path::NormalizePathLayer;

use crate::{
    apps::{AppsCap, with_apps},
    capability::{FoldMount, HasCapTag},
    command::{
        BuildCli, CommandCap, CommandCapability, CommandTag, DispatchCommands, RunCommand,
        ServeCommand, with_commands,
    },
    components::slots::{
        CoreTitle, CoreTitleTag, HeadSlotTag, SlotCap, SlotOf, SlotTag, with_slots,
    },
    config::{
        AppConfig, AppConfigTag, BindTarget, ConfigCap, ConfigCapability, ConfigError, ConfigTag,
        LoadFromToml, with_config,
    },
    db::{DbTag, connect, with_db},
    export::{ExportCap, with_export},
    hooks::{
        FoldAttachState, FoldSeeds, FoldServeStartups, SeedRunner, SeedsCap, SeedsTag,
        ServeStartupRunner, ServeStartupsCap, ServeStartupsTag, StateHooksCap, StateHooksTag,
        with_seeds, with_serve_startups, with_state_hooks,
    },
    http::{
        HttpCap, HttpCapability, HttpTag, MountRoutes, ProvideRequestCaps, into_axum_router,
        with_http,
    },
    migration::{MigrationCap, MigrationCapability, MigrationTag, RunMigrations, with_migrations},
    rune_env::{RuneEnvCap, with_rune_env},
    template::{TemplateCap, with_templates},
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::{GetByCapTag, GetByTag},
        replace::MapByCapTag,
    },
    views::{ViewCap, with_views},
};

/// Builder-phase application holding an HList of capability stores (hooks + items).
///
/// Plugins extend the capability HList during install. Use [`App::mount`] to resolve
/// deferred hooks and produce a [`MountedApp`].
///
/// # Examples
///
/// ```ignore
/// let app = App::new_web_app();
/// let app = lariv_rs::plugins::users::install(app);
/// ```
#[derive(Clone)]
pub struct App<T> {
    /// The typed HList of builder-phase capabilities.
    pub capabilities: T,
}

/// Post-mount application with immutable [`Tagged`](crate::tag::Tagged) capability outputs.
///
/// Created by [`App::mount`]. Safe to share across async tasks once built.
#[derive(Clone)]
pub struct MountedApp<M> {
    /// The typed HList of mounted capability values.
    pub capabilities: M,
}

/// Capability stack returned by [`App::new_web_app`] (database not yet attached).
pub type WebAppCaps = frunk::HList![
    CommandCap<HNil, crate::command::DefaultCommands>,
    ViewCap,
    HttpCap<HNil, HttpCapability<HNil>>,
    SlotCap<
        HNil,
        frunk::HList![crate::tag::Tagged<CoreTitleTag, SlotOf<HeadSlotTag, CoreTitle>>],
    >,
    TemplateCap<HNil, HNil>,
    MigrationCap<HNil, HNil>,
    RuneEnvCap<HNil>,
    ExportCap<HNil>,
    AppsCap<HNil>,
    ServeStartupsCap<HNil>,
    SeedsCap<HNil>,
    StateHooksCap<HNil>,
    ConfigCap<HNil, frunk::HList![crate::tag::Tagged<AppConfigTag, AppConfig>]>,
];

impl App<HNil> {
    /// Create an empty app with no capabilities.
    ///
    /// Prefer [`App::new_web_app`] for a standard web application stack.
    pub fn new() -> Self {
        Self {
            capabilities: hlist![],
        }
    }

    /// Empty app with config, state/seed hooks, apps, migration, template, slots, views, HTTP, CLI.
    pub fn new_web_app() -> App<WebAppCaps> {
        let app = Self::new();
        let app = with_config(app);
        let app = with_state_hooks(app);
        let app = with_seeds(app);
        let app = with_serve_startups(app);
        let app = with_apps(app);
        let app = with_export(app);
        let app = with_rune_env(app);
        let app = with_migrations(app);
        let app = with_templates(app);
        let app = with_slots(app);
        let app = with_http(app);
        let app = with_views(app);
        with_commands(app)
    }
}

impl<L> App<L> {
    /// Attach the shared SeaORM connection.
    pub fn with_db<Proof>(
        self,
        conn: sea_orm::DatabaseConnection,
    ) -> App<HCons<crate::db::DbCap, L>>
    where
        L: HList + CapTagAbsent<DbTag, Proof>,
    {
        with_db(self, conn)
    }

    /// Prepend a builder capability when its tag is not already on the stack.
    pub fn attach_cap<C, Proof>(self, cap: C) -> App<HCons<C, L>>
    where
        C: HasCapTag,
        L: HList + CapTagAbsent<C::Tag, Proof>,
    {
        self.add_capability(cap)
    }

    /// Fold every capability's hooks into its mounted [`Tagged`](crate::tag::Tagged) value.
    ///
    /// Each cell's [`Capability::mount`](crate::capability::Capability::mount) applies that
    /// capability's hooks. The HList may contain any capabilities, including ones attached
    /// by a deployment; this method does not name a fixed stack.
    pub fn mount(self) -> MountedApp<<L as FoldMount>::Output>
    where
        L: FoldMount,
    {
        MountedApp {
            capabilities: self.capabilities.fold_mount(),
        }
    }

    /// Load TOML config, connect the DB, attach plugin state hooks.
    pub async fn load_config<
        CfgIdx,
        Configs,
        AppCfgIdx,
        DbProof,
        StateIdx,
        StateHooks,
        StateProof,
    >(
        self,
        path: impl AsRef<Path>,
    ) -> Result<
        App<<StateHooks as FoldAttachState<HCons<crate::db::DbCap, L>, StateProof>>::Output>,
        ConfigError,
    >
    where
        L: GetByCapTag<ConfigTag, CfgIdx, Value = ConfigCap<HNil, Configs>>,
        L: MapByCapTag<
                ConfigTag,
                ConfigCap<HNil, Configs>,
                CfgIdx,
                OldValue = ConfigCap<HNil, Configs>,
                Output = L,
            >,
        Configs: LoadFromToml + Clone,
        Configs: GetByTag<AppConfigTag, AppCfgIdx, Value = AppConfig>,
        Configs: crate::traits::replace::MapByTag<
                AppConfigTag,
                AppConfig,
                AppCfgIdx,
                OldValue = AppConfig,
                Output = Configs,
            >,
        L: HList + CapTagAbsent<DbTag, DbProof>,
        L: GetByCapTag<StateHooksTag, StateIdx, Value = StateHooksCap<StateHooks>>,
        StateHooks: FoldAttachState<HCons<crate::db::DbCap, L>, StateProof> + Clone,
    {
        let path = path.as_ref();
        let mut items = self.get_capability::<ConfigTag, CfgIdx>().items.clone();
        if path.exists() {
            let raw = std::fs::read_to_string(path)?;
            let root: toml::Value = toml::from_str(&raw)?;
            items.load_from_toml(&root)?;
        }

        items = items.map_by_tag(|mut app_cfg: AppConfig| {
            if let Ok(url) = std::env::var("DATABASE_URL") {
                app_cfg.database_url = url;
            }
            if let Ok(bind) = std::env::var("BIND") {
                app_cfg.bind = Some(bind);
            }
            if let Ok(uds) = std::env::var("UDS") {
                // Empty UDS= must not wipe a TOML `uds` value.
                if !uds.is_empty() {
                    app_cfg.uds = Some(uds);
                }
            }
            app_cfg
        });

        let database_url = <Configs as GetByTag<AppConfigTag, AppCfgIdx>>::get_by_tag(&items)
            .database_url
            .clone();
        let state_hooks = self
            .get_capability::<StateHooksTag, StateIdx>()
            .hooks
            .clone();
        let app = self.replace_capability::<ConfigTag, CfgIdx, _>(|_| {
            crate::capability::CapStore::with_items(items)
        });

        let conn = connect(&database_url)
            .await
            .map_err(|e| ConfigError::Db(e.to_string()))?;
        let app = app.with_db(conn);

        Ok(state_hooks.fold_attach_state(app))
    }
}

impl<M> MountedApp<M> {
    /// Run registered migrators (requires [`DbTag`]).
    pub async fn run_migrations<MigIdx, DbIdx, Migrators>(&self) -> Result<(), DbErr>
    where
        M: GetByTag<MigrationTag, MigIdx, Value = MigrationCapability<Migrators>>,
        M: GetByTag<DbTag, DbIdx, Value = crate::db::DbState>,
        Migrators: RunMigrations + Clone,
    {
        crate::migration::run_migrations(self).await
    }

    /// Record all registered migrations in `seaql_migrations` without running `up`.
    pub async fn mark_migrations<MigIdx, DbIdx, Migrators>(&self) -> Result<usize, DbErr>
    where
        M: GetByTag<MigrationTag, MigIdx, Value = MigrationCapability<Migrators>>,
        M: GetByTag<DbTag, DbIdx, Value = crate::db::DbState>,
        Migrators: crate::migration::CollectMigrations + Clone + Send,
    {
        crate::migration::mark_migrations(self).await
    }

    /// Run every [`RunSeed`](crate::hooks::RunSeed) hook queued during plugin install.
    pub async fn run_seeds<SeedsIdx, Seeds, SeedProof>(&self) -> anyhow::Result<()>
    where
        M: GetByTag<SeedsTag, SeedsIdx, Value = SeedRunner<Seeds>> + Sync,
        Seeds: FoldSeeds<M, SeedProof> + Clone + Send,
    {
        let runner = self.get_capability_output::<SeedsTag, SeedsIdx>().clone();
        runner.seeds.fold_seeds(self).await
    }

    /// Run every [`RunServeStartup`](crate::hooks::RunServeStartup) hook before HTTP listen.
    pub async fn run_serve_startups<ServeIdx, ServeHooks, ServeProof>(&self) -> anyhow::Result<()>
    where
        M: GetByTag<ServeStartupsTag, ServeIdx, Value = ServeStartupRunner<ServeHooks>> + Sync,
        ServeHooks: FoldServeStartups<M, ServeProof> + Clone + Send,
    {
        let runner = self
            .get_capability_output::<ServeStartupsTag, ServeIdx>()
            .clone();
        runner.hooks.fold_serve_startups(self).await
    }

    /// Serve HTTP using [`HttpTag`] routes and [`AppConfig`] bind target (TCP or UDS).
    pub async fn serve<
        CfgIdx,
        Configs,
        AppCfgIdx,
        HttpIdx,
        Routes,
        SlotIdx,
        ServeIdx,
        ServeHooks,
        ServeProof,
    >(
        self,
    ) -> anyhow::Result<()>
    where
        M: GetByTag<ConfigTag, CfgIdx, Value = ConfigCapability<Configs>>,
        Configs: GetByTag<AppConfigTag, AppCfgIdx, Value = AppConfig>,
        M: GetByTag<HttpTag, HttpIdx, Value = std::sync::Arc<HttpCapability<Routes>>>,
        M: GetByTag<SlotTag, SlotIdx, Value = crate::components::SharedChromeFolder>,
        M: GetByTag<ServeStartupsTag, ServeIdx, Value = ServeStartupRunner<ServeHooks>>,
        ServeHooks: FoldServeStartups<M, ServeProof> + Clone + Send,
        M: ProvideRequestCaps + Clone + Send + Sync + 'static,
        Routes: MountRoutes + Clone,
    {
        self.run_serve_startups::<ServeIdx, ServeHooks, ServeProof>()
            .await?;
        let bind = self
            .get_capability_output::<ConfigTag, CfgIdx>()
            .get::<AppConfigTag, AppCfgIdx>()
            .bind_target()?;

        // Bind before building the router. `into_axum_router` clones a large capability
        // HList and can take a long time (or fail); the socket/file must exist as soon as
        // we decide the listen target, matching ops expectations for UDS deployments.
        match bind {
            BindTarget::Tcp(addr) => {
                let listener = tokio::net::TcpListener::bind(addr).await?;
                tracing::info!(%addr, "listening");
                let make_service = {
                    let router = into_axum_router(&self);
                    let service = NormalizePathLayer::trim_trailing_slash().layer(router);
                    Shared::new(service)
                };
                axum::serve(listener, make_service).await?;
            }
            BindTarget::Uds(path) => {
                if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                    std::fs::create_dir_all(parent)?;
                }
                match std::fs::remove_file(&path) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e.into()),
                }
                let listener = tokio::net::UnixListener::bind(&path)?;
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o777))?;
                }
                tracing::info!(path = %path.display(), "listening");
                let make_service = {
                    let router = into_axum_router(&self);
                    let service = NormalizePathLayer::trim_trailing_slash().layer(router);
                    Shared::new(service)
                };
                axum::serve(listener, make_service).await?;
            }
        }
        Ok(())
    }

    /// Parse CLI args and dispatch a registered command (defaults to `serve`).
    pub async fn run<CmdIdx, Cmds, DispatchProof, ServeProof>(self) -> anyhow::Result<()>
    where
        M: GetByTag<CommandTag, CmdIdx, Value = CommandCapability<Cmds>> + Send + 'static,
        Cmds: BuildCli<M, DispatchProof> + DispatchCommands<M, DispatchProof> + Clone + Send,
        CmdIdx: Send + Sync + 'static,
        ServeCommand: RunCommand<M, ServeProof>,
        <ServeCommand as RunCommand<M, ServeProof>>::Args: Default,
    {
        let cli = self
            .get_capability_output::<CommandTag, CmdIdx>()
            .build_cli();
        let matches = cli.get_matches();
        match matches.subcommand() {
            Some((name, sub_matches)) => {
                let cmds = self
                    .get_capability_output::<CommandTag, CmdIdx>()
                    .commands
                    .clone();
                cmds.dispatch(name, sub_matches, self).await
            }
            None => {
                let args = <ServeCommand as RunCommand<M, ServeProof>>::Args::default();
                <ServeCommand as RunCommand<M, ServeProof>>::run(args, self).await
            }
        }
    }
}

impl Default for App<HNil> {
    fn default() -> Self {
        Self::new()
    }
}
