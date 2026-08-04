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
//! | Builder | [`App::mount`] | Resolve hooks and fold to mounted capabilities |
//! | Mounted | [`MountedApp::run_migrations`] | Apply SeaORM migrations from all plugins |
//! | Mounted | [`MountedApp::run_seeds`] | Run startup seed hooks |
//! | Mounted | [`MountedApp::serve`] | Start Axum HTTP server |
//! | Mounted | [`MountedApp::run`] | Parse CLI and dispatch command (default: serve) |

use std::net::SocketAddr;
use std::path::Path;

use frunk::{HCons, HNil, hlist, hlist::HList};
use sea_orm::DbErr;
use tower::{Layer, make::Shared};
use tower_http::normalize_path::NormalizePathLayer;

use crate::{
    apps::{AppsCap, AppsTag, with_apps},
    export::{ExportCap, ExportTag, with_export},
    capability::{FoldMount, HasCapTag},
    grapesjs::{GrapesJsCap, GrapesJsTag, with_grapesjs},
    llm_tools::{LlmToolsCap, LlmToolsTag, with_llm_tools},
    rune_env::{RuneEnvCap, RuneEnvTag, with_rune_env},
    command::{
        BuildCli, CommandCap, CommandCapability, CommandTag, DispatchCommands, RunCommand,
        ServeCommand, with_commands,
    },
    components::slots::{
        CoreTitle, CoreTitleTag, HeadSlotTag, SlotCap, SlotOf, SlotTag, with_slots,
    },
    config::{
        AppConfig, AppConfigTag, ConfigCap, ConfigCapability, ConfigError, ConfigTag, LoadFromToml,
        with_config,
    },
    db::{DbTag, connect, with_db},
    hooks::{
        FoldAttachState, FoldSeeds, SeedRunner, SeedsCap, SeedsTag, StateHooksCap, StateHooksTag,
        with_seeds, with_state_hooks,
    },
    http::{
        FoldMountRoutes, HttpCap, HttpCapability, HttpTag, MountRoutes, ProvideRequestCaps,
        into_axum_router, with_http,
    },
    migration::{MigrationCap, MigrationCapability, MigrationTag, RunMigrations, with_migrations},
    template::{TemplateCap, TemplateTag, with_templates},
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
    LlmToolsCap<HNil>,
    GrapesJsCap<HNil>,
    ExportCap<HNil>,
    AppsCap<HNil>,
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

    /// Empty app with config, state/seed hooks, apps, grapesjs, llm tools, migration, template, slots, views, HTTP, CLI.
    pub fn new_web_app() -> App<WebAppCaps> {
        let app = Self::new();
        let app = with_config(app);
        let app = with_state_hooks(app);
        let app = with_seeds(app);
        let app = with_apps(app);
        let app = with_export(app);
        let app = with_grapesjs(app);
        let app = with_llm_tools(app);
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
    pub fn with_db<Proof>(self, conn: sea_orm::DatabaseConnection) -> App<HCons<crate::db::DbCap, L>>
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

    /// Apply deferred registry hooks, HTTP route hooks, then fold every capability to [`MountedApp`].
    pub fn mount<
        TplIdx,
        TplHooks,
        TplItems,
        SlotIdx,
        SlotHooks,
        SlotItems,
        MigIdx,
        MigHooks,
        MigItems,
        CmdIdx,
        CmdHooks,
        CmdItems,
        AppsIdx,
        AppsHooks,
        AppsProof,
        ExportIdx,
        ExportHooks,
        ExportProof,
        GjsIdx,
        GjsHooks,
        GjsProof,
        ToolsIdx,
        ToolsHooks,
        ToolsProof,
        RuneEnvIdx,
        RuneEnvHooks,
        RuneEnvProof,
        HttpIdx,
        HttpHooks,
        HttpRoutes,
        Proof,
    >(
        self,
    ) -> MountedApp<
        <AfterHttpResolve<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
            CmdIdx,
            CmdHooks,
            CmdItems,
            AppsIdx,
            ExportIdx,
            GjsIdx,
            ToolsIdx,
            RuneEnvIdx,
            HttpIdx,
            HttpHooks,
            HttpRoutes,
            Proof,
        > as FoldMount>::Output,
    >
    where
        L: GetByCapTag<TemplateTag, TplIdx, Value = TemplateCap<TplHooks, TplItems>>,
        L: MapByCapTag<
                TemplateTag,
                TemplateCap<HNil, <TplHooks as crate::capability::FoldRegistrarHooks<crate::template::TemplateTag, TplItems>>::Output>,
                TplIdx,
                OldValue = TemplateCap<TplHooks, TplItems>,
            >,
        TplHooks: crate::capability::FoldRegistrarHooks<crate::template::TemplateTag, TplItems>,
        AfterTemplates<L, TplIdx, TplHooks, TplItems>: GetByCapTag<
                SlotTag,
                SlotIdx,
                Value = SlotCap<SlotHooks, SlotItems>,
            >,
        AfterTemplates<L, TplIdx, TplHooks, TplItems>: MapByCapTag<
                SlotTag,
                SlotCap<HNil, <SlotHooks as crate::capability::FoldRegistrarHooks<crate::components::SlotTag, SlotItems>>::Output>,
                SlotIdx,
                OldValue = SlotCap<SlotHooks, SlotItems>,
            >,
        SlotHooks: crate::capability::FoldRegistrarHooks<crate::components::SlotTag, SlotItems>,
        AfterSlots<L, TplIdx, TplHooks, TplItems, SlotIdx, SlotHooks, SlotItems>: GetByCapTag<
                MigrationTag,
                MigIdx,
                Value = MigrationCap<MigHooks, MigItems>,
            >,
        AfterSlots<L, TplIdx, TplHooks, TplItems, SlotIdx, SlotHooks, SlotItems>: MapByCapTag<
                MigrationTag,
                MigrationCap<HNil, <MigHooks as crate::capability::FoldRegistrarHooks<crate::migration::MigrationTag, MigItems>>::Output>,
                MigIdx,
                OldValue = MigrationCap<MigHooks, MigItems>,
            >,
        MigHooks: crate::capability::FoldRegistrarHooks<crate::migration::MigrationTag, MigItems>,
        AfterMigrations<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
        >: GetByCapTag<CommandTag, CmdIdx, Value = CommandCap<CmdHooks, CmdItems>>,
        AfterMigrations<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
        >: MapByCapTag<
                CommandTag,
                CommandCap<HNil, <CmdHooks as crate::capability::FoldRegistrarHooks<crate::command::CommandTag, CmdItems>>::Output>,
                CmdIdx,
                OldValue = CommandCap<CmdHooks, CmdItems>,
            >,
        CmdHooks: crate::capability::FoldRegistrarHooks<crate::command::CommandTag, CmdItems>,
        AfterCommands<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
            CmdIdx,
            CmdHooks,
            CmdItems,
        >: GetByCapTag<AppsTag, AppsIdx, Value = AppsCap<AppsHooks>>,
        AfterCommands<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
            CmdIdx,
            CmdHooks,
            CmdItems,
        >: MapByCapTag<
                AppsTag,
                AppsCap<HNil>,
                AppsIdx,
                OldValue = AppsCap<AppsHooks>,
            >,
        AppsHooks: crate::capability::ApplyHooks<crate::apps::AppsCapability, AppsProof, Output = crate::apps::AppsCapability>,
        AfterApps<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
            CmdIdx,
            CmdHooks,
            CmdItems,
            AppsIdx,
        >: GetByCapTag<ExportTag, ExportIdx, Value = ExportCap<ExportHooks>>,
        AfterApps<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
            CmdIdx,
            CmdHooks,
            CmdItems,
            AppsIdx,
        >: MapByCapTag<
                ExportTag,
                ExportCap<HNil>,
                ExportIdx,
                OldValue = ExportCap<ExportHooks>,
            >,
        ExportHooks: crate::capability::ApplyHooks<crate::export::ExportCapability, ExportProof, Output = crate::export::ExportCapability>,
        AfterExport<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
            CmdIdx,
            CmdHooks,
            CmdItems,
            AppsIdx,
            ExportIdx,
        >: GetByCapTag<GrapesJsTag, GjsIdx, Value = GrapesJsCap<GjsHooks>>,
        AfterExport<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
            CmdIdx,
            CmdHooks,
            CmdItems,
            AppsIdx,
            ExportIdx,
        >: MapByCapTag<
                GrapesJsTag,
                GrapesJsCap<HNil>,
                GjsIdx,
                OldValue = GrapesJsCap<GjsHooks>,
            >,
        GjsHooks: crate::capability::ApplyHooks<
                crate::grapesjs::GrapesJsCapability,
                GjsProof,
                Output = crate::grapesjs::GrapesJsCapability,
            >,
        AfterGrapesJs<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
            CmdIdx,
            CmdHooks,
            CmdItems,
            AppsIdx,
            ExportIdx,
            GjsIdx,
        >: GetByCapTag<LlmToolsTag, ToolsIdx, Value = LlmToolsCap<ToolsHooks>>,
        AfterGrapesJs<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
            CmdIdx,
            CmdHooks,
            CmdItems,
            AppsIdx,
            ExportIdx,
            GjsIdx,
        >: MapByCapTag<
                LlmToolsTag,
                LlmToolsCap<HNil>,
                ToolsIdx,
                OldValue = LlmToolsCap<ToolsHooks>,
            >,
        ToolsHooks: crate::capability::ApplyHooks<
                crate::llm_tools::LlmToolsCapability,
                ToolsProof,
                Output = crate::llm_tools::LlmToolsCapability,
            >,
        AfterTools<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
            CmdIdx,
            CmdHooks,
            CmdItems,
            AppsIdx,
            ExportIdx,
            GjsIdx,
            ToolsIdx,
        >: GetByCapTag<RuneEnvTag, RuneEnvIdx, Value = RuneEnvCap<RuneEnvHooks>>,
        AfterTools<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
            CmdIdx,
            CmdHooks,
            CmdItems,
            AppsIdx,
            ExportIdx,
            GjsIdx,
            ToolsIdx,
        >: MapByCapTag<
                RuneEnvTag,
                RuneEnvCap<HNil>,
                RuneEnvIdx,
                OldValue = RuneEnvCap<RuneEnvHooks>,
            >,
        RuneEnvHooks: crate::capability::ApplyHooks<
                crate::rune_env::RuneEnvCapability,
                RuneEnvProof,
                Output = crate::rune_env::RuneEnvCapability,
            >,
        AfterRuneEnv<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
            CmdIdx,
            CmdHooks,
            CmdItems,
            AppsIdx,
            ExportIdx,
            GjsIdx,
            ToolsIdx,
            RuneEnvIdx,
        >: GetByCapTag<
                HttpTag,
                HttpIdx,
                Value = HttpCap<HttpHooks, HttpCapability<HttpRoutes>>,
            >,
        AfterRuneEnv<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
            CmdIdx,
            CmdHooks,
            CmdItems,
            AppsIdx,
            ExportIdx,
            GjsIdx,
            ToolsIdx,
            RuneEnvIdx,
        >: MapByCapTag<
                HttpTag,
                HttpCap<
                    HNil,
                    <HttpHooks as FoldMountRoutes<HttpCapability<HttpRoutes>, Proof>>::Output,
                >,
                HttpIdx,
                OldValue = HttpCap<HttpHooks, HttpCapability<HttpRoutes>>,
            >,
        HttpHooks: FoldMountRoutes<HttpCapability<HttpRoutes>, Proof>,
        AfterHttpResolve<
            L,
            TplIdx,
            TplHooks,
            TplItems,
            SlotIdx,
            SlotHooks,
            SlotItems,
            MigIdx,
            MigHooks,
            MigItems,
            CmdIdx,
            CmdHooks,
            CmdItems,
            AppsIdx,
            ExportIdx,
            GjsIdx,
            ToolsIdx,
            RuneEnvIdx,
            HttpIdx,
            HttpHooks,
            HttpRoutes,
            Proof,
        >: FoldMount,
    {
        let app = self.replace_capability::<TemplateTag, TplIdx, _>(|c: TemplateCap<TplHooks, TplItems>| {
            c.resolve_hooks()
        });
        let app = app.replace_capability::<SlotTag, SlotIdx, _>(|c: SlotCap<SlotHooks, SlotItems>| {
            c.resolve_hooks()
        });
        let app = app.replace_capability::<MigrationTag, MigIdx, _>(
            |c: MigrationCap<MigHooks, MigItems>| c.resolve_hooks(),
        );
        let app = app.replace_capability::<CommandTag, CmdIdx, _>(
            |c: CommandCap<CmdHooks, CmdItems>| c.resolve_hooks(),
        );
        let app = app.replace_capability::<AppsTag, AppsIdx, _>(|c: AppsCap<AppsHooks>| {
            c.resolve_hooks()
        });
        let app = app.replace_capability::<ExportTag, ExportIdx, _>(|c: ExportCap<ExportHooks>| {
            c.resolve_hooks()
        });
        let app = app.replace_capability::<GrapesJsTag, GjsIdx, _>(|c: GrapesJsCap<GjsHooks>| {
            c.resolve_hooks()
        });
        let app = app.replace_capability::<LlmToolsTag, ToolsIdx, _>(|c: LlmToolsCap<ToolsHooks>| {
            c.resolve_hooks()
        });
        let app = app.replace_capability::<RuneEnvTag, RuneEnvIdx, _>(|c: RuneEnvCap<RuneEnvHooks>| {
            c.resolve_hooks()
        });
        let app = app.replace_capability::<HttpTag, HttpIdx, _>(
            |c: HttpCap<HttpHooks, HttpCapability<HttpRoutes>>| c.resolve_route_hooks::<Proof>(),
        );
        MountedApp {
            capabilities: app.capabilities.fold_mount(),
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
            app_cfg
        });

        let database_url = <Configs as GetByTag<AppConfigTag, AppCfgIdx>>::get_by_tag(&items)
            .database_url
            .clone();
        let state_hooks = self.get_capability::<StateHooksTag, StateIdx>().hooks.clone();
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

// Helper aliases for mount type chain — keep app.rs readable.
type AfterTemplates<L, TplIdx, TplHooks, TplItems> = <L as MapByCapTag<
    TemplateTag,
    TemplateCap<HNil, <TplHooks as crate::capability::FoldRegistrarHooks<crate::template::TemplateTag, TplItems>>::Output>,
    TplIdx,
>>::Output;

type AfterSlots<L, TplIdx, TplHooks, TplItems, SlotIdx, SlotHooks, SlotItems> =
    <AfterTemplates<L, TplIdx, TplHooks, TplItems> as MapByCapTag<
        SlotTag,
        SlotCap<HNil, <SlotHooks as crate::capability::FoldRegistrarHooks<crate::components::SlotTag, SlotItems>>::Output>,
        SlotIdx,
    >>::Output;

type AfterMigrations<
    L,
    TplIdx,
    TplHooks,
    TplItems,
    SlotIdx,
    SlotHooks,
    SlotItems,
    MigIdx,
    MigHooks,
    MigItems,
> = <AfterSlots<L, TplIdx, TplHooks, TplItems, SlotIdx, SlotHooks, SlotItems> as MapByCapTag<
    MigrationTag,
    MigrationCap<HNil, <MigHooks as crate::capability::FoldRegistrarHooks<crate::migration::MigrationTag, MigItems>>::Output>,
    MigIdx,
>>::Output;

type AfterCommands<
    L,
    TplIdx,
    TplHooks,
    TplItems,
    SlotIdx,
    SlotHooks,
    SlotItems,
    MigIdx,
    MigHooks,
    MigItems,
    CmdIdx,
    CmdHooks,
    CmdItems,
> = <AfterMigrations<
    L,
    TplIdx,
    TplHooks,
    TplItems,
    SlotIdx,
    SlotHooks,
    SlotItems,
    MigIdx,
    MigHooks,
    MigItems,
> as MapByCapTag<
    CommandTag,
    CommandCap<HNil, <CmdHooks as crate::capability::FoldRegistrarHooks<crate::command::CommandTag, CmdItems>>::Output>,
    CmdIdx,
>>::Output;

type AfterApps<
    L,
    TplIdx,
    TplHooks,
    TplItems,
    SlotIdx,
    SlotHooks,
    SlotItems,
    MigIdx,
    MigHooks,
    MigItems,
    CmdIdx,
    CmdHooks,
    CmdItems,
    AppsIdx,
> = <AfterCommands<
    L,
    TplIdx,
    TplHooks,
    TplItems,
    SlotIdx,
    SlotHooks,
    SlotItems,
    MigIdx,
    MigHooks,
    MigItems,
    CmdIdx,
    CmdHooks,
    CmdItems,
> as MapByCapTag<AppsTag, AppsCap<HNil>, AppsIdx>>::Output;

type AfterExport<
    L,
    TplIdx,
    TplHooks,
    TplItems,
    SlotIdx,
    SlotHooks,
    SlotItems,
    MigIdx,
    MigHooks,
    MigItems,
    CmdIdx,
    CmdHooks,
    CmdItems,
    AppsIdx,
    ExportIdx,
> = <AfterApps<
    L,
    TplIdx,
    TplHooks,
    TplItems,
    SlotIdx,
    SlotHooks,
    SlotItems,
    MigIdx,
    MigHooks,
    MigItems,
    CmdIdx,
    CmdHooks,
    CmdItems,
    AppsIdx,
> as MapByCapTag<ExportTag, ExportCap<HNil>, ExportIdx>>::Output;

type AfterGrapesJs<
    L,
    TplIdx,
    TplHooks,
    TplItems,
    SlotIdx,
    SlotHooks,
    SlotItems,
    MigIdx,
    MigHooks,
    MigItems,
    CmdIdx,
    CmdHooks,
    CmdItems,
    AppsIdx,
    ExportIdx,
    GjsIdx,
> = <AfterExport<
    L,
    TplIdx,
    TplHooks,
    TplItems,
    SlotIdx,
    SlotHooks,
    SlotItems,
    MigIdx,
    MigHooks,
    MigItems,
    CmdIdx,
    CmdHooks,
    CmdItems,
    AppsIdx,
    ExportIdx,
> as MapByCapTag<GrapesJsTag, GrapesJsCap<HNil>, GjsIdx>>::Output;

type AfterTools<
    L,
    TplIdx,
    TplHooks,
    TplItems,
    SlotIdx,
    SlotHooks,
    SlotItems,
    MigIdx,
    MigHooks,
    MigItems,
    CmdIdx,
    CmdHooks,
    CmdItems,
    AppsIdx,
    ExportIdx,
    GjsIdx,
    ToolsIdx,
> = <AfterGrapesJs<
    L,
    TplIdx,
    TplHooks,
    TplItems,
    SlotIdx,
    SlotHooks,
    SlotItems,
    MigIdx,
    MigHooks,
    MigItems,
    CmdIdx,
    CmdHooks,
    CmdItems,
    AppsIdx,
    ExportIdx,
    GjsIdx,
> as MapByCapTag<LlmToolsTag, LlmToolsCap<HNil>, ToolsIdx>>::Output;

type AfterRuneEnv<
    L,
    TplIdx,
    TplHooks,
    TplItems,
    SlotIdx,
    SlotHooks,
    SlotItems,
    MigIdx,
    MigHooks,
    MigItems,
    CmdIdx,
    CmdHooks,
    CmdItems,
    AppsIdx,
    ExportIdx,
    GjsIdx,
    ToolsIdx,
    RuneEnvIdx,
> = <AfterTools<
    L,
    TplIdx,
    TplHooks,
    TplItems,
    SlotIdx,
    SlotHooks,
    SlotItems,
    MigIdx,
    MigHooks,
    MigItems,
    CmdIdx,
    CmdHooks,
    CmdItems,
    AppsIdx,
    ExportIdx,
    GjsIdx,
    ToolsIdx,
> as MapByCapTag<RuneEnvTag, RuneEnvCap<HNil>, RuneEnvIdx>>::Output;

type AfterHttpResolve<
    L,
    TplIdx,
    TplHooks,
    TplItems,
    SlotIdx,
    SlotHooks,
    SlotItems,
    MigIdx,
    MigHooks,
    MigItems,
    CmdIdx,
    CmdHooks,
    CmdItems,
    AppsIdx,
    ExportIdx,
    GjsIdx,
    ToolsIdx,
    RuneEnvIdx,
    HttpIdx,
    HttpHooks,
    HttpRoutes,
    Proof,
> = <AfterRuneEnv<
    L,
    TplIdx,
    TplHooks,
    TplItems,
    SlotIdx,
    SlotHooks,
    SlotItems,
    MigIdx,
    MigHooks,
    MigItems,
    CmdIdx,
    CmdHooks,
    CmdItems,
    AppsIdx,
    ExportIdx,
    GjsIdx,
    ToolsIdx,
    RuneEnvIdx,
> as MapByCapTag<
    HttpTag,
    HttpCap<
        HNil,
        <HttpHooks as FoldMountRoutes<HttpCapability<HttpRoutes>, Proof>>::Output,
    >,
    HttpIdx,
>>::Output;

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

    /// Run every [`RunSeed`](crate::hooks::RunSeed) hook queued during plugin install.
    pub async fn run_seeds<SeedsIdx, Seeds, SeedProof>(&self) -> anyhow::Result<()>
    where
        M: GetByTag<SeedsTag, SeedsIdx, Value = SeedRunner<Seeds>> + Sync,
        Seeds: FoldSeeds<M, SeedProof> + Clone + Send,
    {
        let runner = self.get_capability_output::<SeedsTag, SeedsIdx>().clone();
        runner.seeds.fold_seeds(self).await
    }

    /// Serve HTTP using [`HttpTag`] routes and [`AppConfig`] bind address.
    pub async fn serve<CfgIdx, Configs, AppCfgIdx, HttpIdx, Routes, SlotIdx>(
        self,
    ) -> anyhow::Result<()>
    where
        M: GetByTag<ConfigTag, CfgIdx, Value = ConfigCapability<Configs>>,
        Configs: GetByTag<AppConfigTag, AppCfgIdx, Value = AppConfig>,
        M: GetByTag<HttpTag, HttpIdx, Value = std::sync::Arc<HttpCapability<Routes>>>,
        M: GetByTag<SlotTag, SlotIdx, Value = crate::components::SharedChromeFolder>,
        M: ProvideRequestCaps + Clone + Send + Sync + 'static,
        Routes: MountRoutes + Clone,
    {
        let bind = self
            .get_capability_output::<ConfigTag, CfgIdx>()
            .get::<AppConfigTag, AppCfgIdx>()
            .bind_addr()
            .to_string();
        let router = into_axum_router(&self);
        let service = NormalizePathLayer::trim_trailing_slash().layer(router);
        let addr: SocketAddr = bind.parse()?;
        tracing::info!(%addr, "listening");
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, Shared::new(service)).await?;
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
        let cli = self.get_capability_output::<CommandTag, CmdIdx>().build_cli();
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
