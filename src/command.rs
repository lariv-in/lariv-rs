//! CLI command registry capability — plugins register subcommands; clap builds and dispatches them.
//!
//! The command capability holds a compile-time HList of tagged [`RunCommand`] implementations.
//! At mount, plugin [`CommandRegistrar`] hooks prepend commands. Built-in migrate, seed, and
//! serve commands are included by [`with_commands`].
//!
//! # Lifecycle
//!
//! 1. Attach [`with_commands`] (or an empty capability and register manually).
//! 2. Plugins queue [`CommandRegistrar`] hooks during install.
//! 3. At mount, hooks fold over the command HList → [`CommandCapability`].
//! 4. [`CommandCapability::build_cli`] produces a clap root; [`DispatchCommands::dispatch`] routes argv.
//!
//! # Core types
//!
//! - [`CommandTag`] — capability tag
//! - [`CommandCapability`] — mounted HList of tagged commands
//! - [`CommandCap`] — builder-phase [`CapStore`]
//! - [`RunCommand`] — clap metadata + async runner for one subcommand
//! - [`CommandRegistrar`] — plugin hook trait
//! - [`BuildCli`] / [`DispatchCommands`] — fold traits for clap integration
//!
//! # Built-in commands
//!
//! - [`MigrateCommand`] — runs [`crate::migration::RunMigrations`]
//! - [`SeedCommand`] — runs [`crate::hooks::FoldSeeds`]
//! - [`ServeCommand`] — starts the HTTP server
//!
//! # Examples
//!
//! ```rust ignore
//! // Plugin command registration (implement CommandRegistrar manually or via macro):
//! impl<C> CommandRegistrar<C> for MyRegisterHook {
//!     type Output = impl HList;
//!     fn register_commands(self, cap: CommandCapability<C>) -> CommandCapability<Self::Output> {
//!         cap.prepend::<MyCmdTag, _>(MyCommand)
//!     }
//! }
//!
//! let app = with_commands(app);
//! ```

use clap::{ArgMatches, Args, Command as ClapCommand, FromArgMatches};
use frunk::{HCons, HNil, hlist::HList};

use crate::{
    app::{App, MountedApp},
    capability::{
        ApplyHooks, CapStore, Capability, FoldRegistrarHooks, apply_registrar_hook,
        mount_with_hooks,
    },
    components::SlotTag,
    config::{AppConfig, AppConfigTag, ConfigCapability, ConfigTag},
    db::{DbState, DbTag},
    hooks::{FoldSeeds, SeedRunner, SeedsTag},
    http::{HttpCapability, HttpTag, MountRoutes, ProvideRequestCaps},
    migration::{MigrationCapability, MigrationTag, RunMigrations, mark_migrations},
    tag::Tagged,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::GetByTag,
    },
};

/// Capability tag for the CLI command registry.
pub struct CommandTag;

/// Tag for the built-in [`MigrateCommand`].
pub struct MigrateCommandTag;

/// Tag for the built-in [`SeedCommand`].
pub struct SeedCommandTag;

/// Tag for the built-in [`MarkMigrationsCommand`].
pub struct MarkMigrationsCommandTag;

/// Tag for the built-in [`ServeCommand`].
pub struct ServeCommandTag;

/// Plugin hook for appending commands onto a [`CommandCapability`].
pub trait CommandRegistrar<C>: Sized {
    type Output;
    fn register_commands(self, cap: CommandCapability<C>) -> CommandCapability<Self::Output>;
}

/// Registered CLI subcommand: clap metadata + async runner.
///
/// Implement on a zero-sized or cloneable type; clap args are a separate [`Args`] struct.
///
/// # Examples
///
/// ```rust ignore
/// #[derive(Args, Clone, Default)]
/// struct MyArgs { #[arg(long)] verbose: bool }
///
/// struct MyCommand;
///
/// #[async_trait]
/// impl<M> RunCommand<M> for MyCommand {
///     type Args = MyArgs;
///     const NAME: &'static str = "my-cmd";
///     const ABOUT: &'static str = "Do something useful";
///     async fn run(args: Self::Args, app: MountedApp<M>) -> anyhow::Result<()> {
///         Ok(())
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait RunCommand<M, Proof = ()>: Sized {
    type Args: Args + FromArgMatches + Clone + Send;

    const NAME: &'static str;
    const ABOUT: &'static str;

    async fn run(args: Self::Args, app: MountedApp<M>) -> anyhow::Result<()>;
}

/// Fold command HList into a clap [`ClapCommand`] (tail first = registration order).
pub trait BuildCli<M, Proof = ()> {
    fn augment_cli(cmd: ClapCommand) -> ClapCommand;
}

impl<M> BuildCli<M> for HNil {
    fn augment_cli(cmd: ClapCommand) -> ClapCommand {
        cmd
    }
}

impl<Tag, C, Tail, M, TailProof, Proof> BuildCli<M, (TailProof, Proof)>
    for HCons<Tagged<Tag, C>, Tail>
where
    C: RunCommand<M, Proof>,
    Tail: BuildCli<M, TailProof>,
{
    fn augment_cli(cmd: ClapCommand) -> ClapCommand {
        let cmd = Tail::augment_cli(cmd);
        let sub = C::Args::augment_args(ClapCommand::new(C::NAME).about(C::ABOUT));
        cmd.subcommand(sub)
    }
}

/// Dispatch argv to a registered command's [`RunCommand::run`].
#[async_trait::async_trait]
pub trait DispatchCommands<M, Proof = ()>: Sized {
    async fn dispatch(
        self,
        name: &str,
        matches: &ArgMatches,
        app: MountedApp<M>,
    ) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl<M> DispatchCommands<M> for HNil
where
    M: Send + 'static,
{
    async fn dispatch(self, name: &str, _: &ArgMatches, _: MountedApp<M>) -> anyhow::Result<()> {
        anyhow::bail!("unknown command: {name}")
    }
}

#[async_trait::async_trait]
impl<Tag, C, Tail, M, TailProof, Proof> DispatchCommands<M, (TailProof, Proof)>
    for HCons<Tagged<Tag, C>, Tail>
where
    Tag: Send + Sync + 'static,
    C: RunCommand<M, Proof> + Send + Sync,
    Tail: DispatchCommands<M, TailProof> + Send,
    M: Send + 'static,
{
    async fn dispatch(
        self,
        name: &str,
        matches: &ArgMatches,
        app: MountedApp<M>,
    ) -> anyhow::Result<()> {
        if name == C::NAME {
            let args = C::Args::from_arg_matches(matches)?;
            <C as RunCommand<M, Proof>>::run(args, app).await
        } else {
            self.tail.dispatch(name, matches, app).await
        }
    }
}

/// Mounted CLI command capability.
#[derive(Clone)]
pub struct CommandCapability<Cmds> {
    pub commands: Cmds,
}

impl CommandCapability<HNil> {
    /// Empty command list (starting point for [`CommandRegistrar`] hooks).
    pub fn new() -> Self {
        Self { commands: HNil }
    }
}

impl Default for CommandCapability<HNil> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Cmds> CommandCapability<Cmds> {
    /// Prepend a tagged command (head of the HList = most recently registered).
    pub fn prepend<Tag, C>(self, command: C) -> CommandCapability<HCons<Tagged<Tag, C>, Cmds>>
    where
        Cmds: HList,
    {
        CommandCapability {
            commands: HCons {
                head: Tagged::new(command),
                tail: self.commands,
            },
        }
    }

    /// Build the root clap command (`lariv`) with all registered subcommands.
    pub fn build_cli<M, Proof>(&self) -> ClapCommand
    where
        Cmds: BuildCli<M, Proof>,
    {
        let cmd = ClapCommand::new("lariv")
            .subcommand_required(false)
            .arg_required_else_help(false);
        Cmds::augment_cli(cmd)
    }
}

/// Builder-phase command capability.
pub type CommandCap<Hooks, Items> = CapStore<CommandTag, Hooks, Items>;

impl<Hooks, Items> CommandCap<Hooks, Items> {
    /// Eagerly fold registrar hooks into items (testing / pre-mount inspection).
    pub fn resolve_hooks(
        self,
    ) -> CommandCap<HNil, <Hooks as FoldRegistrarHooks<CommandTag, Items>>::Output>
    where
        Hooks: FoldRegistrarHooks<CommandTag, Items>,
    {
        CapStore::with_items(self.hooks.fold_registrar_hooks(self.items))
    }
}

apply_registrar_hook! {
    capability: CommandCapability;
    trait: CommandRegistrar;
    method: register_commands;
    field: commands;
    proof: crate::capability::CommandHookProof;
    tag: CommandTag;
}

impl<Hooks, Items> Capability for CommandCap<Hooks, Items>
where
    Hooks: ApplyHooks<Items>,
{
    type Value = CommandCapability<Hooks::Output>;
    type Output = Tagged<CommandTag, CommandCapability<Hooks::Output>>;
    type Hooks = Hooks;
    type Items = Items;

    fn mount(self) -> Self::Output {
        mount_with_hooks(self, |items| CommandCapability { commands: items })
    }
}

/// Default command HList from [`with_commands`] (migrate, mark-migrations, seed, serve).
pub type DefaultCommands = HCons<
    Tagged<ServeCommandTag, ServeCommand>,
    HCons<
        Tagged<SeedCommandTag, SeedCommand>,
        HCons<
            Tagged<MarkMigrationsCommandTag, MarkMigrationsCommand>,
            HCons<
                Tagged<MigrateCommandTag, MigrateCommand>,
                HNil,
            >,
        >,
    >,
>;

/// Run database migrations (`lariv migrate`).
#[derive(Clone, Copy, Debug, Default)]
pub struct MigrateCommand;

/// CLI args for [`MigrateCommand`] (no flags).
#[derive(Args, Debug, Clone, Default)]
pub struct MigrateArgs {}

#[async_trait::async_trait]
impl<M, MigIdx, DbIdx, Migrators> RunCommand<M, (MigIdx, DbIdx, Migrators)> for MigrateCommand
where
    M: GetByTag<MigrationTag, MigIdx, Value = MigrationCapability<Migrators>>
        + GetByTag<DbTag, DbIdx, Value = DbState>
        + Sync
        + Send
        + 'static,
    Migrators: RunMigrations + Clone + Send + Sync,
    MigIdx: Send + Sync + 'static,
    DbIdx: Send + Sync + 'static,
{
    type Args = MigrateArgs;
    const NAME: &'static str = "migrate";
    const ABOUT: &'static str = "Run database migrations";

    async fn run(_args: Self::Args, app: MountedApp<M>) -> anyhow::Result<()> {
        app.run_migrations().await?;
        Ok(())
    }
}

/// Mark every registered migration as applied without running DDL (`lariv mark-migrations`).
#[derive(Clone, Copy, Debug, Default)]
pub struct MarkMigrationsCommand;

/// CLI args for [`MarkMigrationsCommand`] (no flags).
#[derive(Args, Debug, Clone, Default)]
pub struct MarkMigrationsArgs {}

#[async_trait::async_trait]
impl<M, MigIdx, DbIdx, Migrators> RunCommand<M, (MigIdx, DbIdx, Migrators)> for MarkMigrationsCommand
where
    M: GetByTag<MigrationTag, MigIdx, Value = MigrationCapability<Migrators>>
        + GetByTag<DbTag, DbIdx, Value = crate::db::DbState>
        + Sync
        + Send
        + 'static,
    Migrators: crate::migration::CollectMigrations + Clone + Send + Sync,
    MigIdx: Send + Sync + 'static,
    DbIdx: Send + Sync + 'static,
{
    type Args = MarkMigrationsArgs;
    const NAME: &'static str = "mark-migrations";
    const ABOUT: &'static str =
        "Mark all registered migrations as applied without running them";

    async fn run(_args: Self::Args, app: MountedApp<M>) -> anyhow::Result<()> {
        let inserted = mark_migrations(&app).await?;
        tracing::info!(inserted, "migration versions recorded in seaql_migrations");
        Ok(())
    }
}

/// Run registered seed hooks (`lariv seed`).
#[derive(Clone, Copy, Debug, Default)]
pub struct SeedCommand;

/// CLI args for [`SeedCommand`] (no flags).
#[derive(Args, Debug, Clone, Default)]
pub struct SeedArgs {}

#[async_trait::async_trait]
impl<M, SeedsIdx, Seeds, SeedProof> RunCommand<M, (SeedsIdx, Seeds, SeedProof)> for SeedCommand
where
    M: GetByTag<SeedsTag, SeedsIdx, Value = SeedRunner<Seeds>> + Sync + Send + 'static,
    Seeds: FoldSeeds<M, SeedProof> + Clone + Send + Sync,
    SeedsIdx: Send + Sync + 'static,
    SeedProof: Send + Sync + 'static,
{
    type Args = SeedArgs;
    const NAME: &'static str = "seed";
    const ABOUT: &'static str = "Run database seed hooks";

    async fn run(_args: Self::Args, app: MountedApp<M>) -> anyhow::Result<()> {
        app.run_seeds().await
    }
}

/// Start the HTTP server (`lariv serve`).
#[derive(Clone, Copy, Debug, Default)]
pub struct ServeCommand;

/// CLI args for [`ServeCommand`] (no flags).
#[derive(Args, Debug, Clone, Default)]
pub struct ServeArgs {}

#[async_trait::async_trait]
impl<M, CfgIdx, Configs, AppCfgIdx, HttpIdx, Routes, SlotIdx>
    RunCommand<M, (CfgIdx, Configs, AppCfgIdx, HttpIdx, Routes, SlotIdx)> for ServeCommand
where
    M: GetByTag<ConfigTag, CfgIdx, Value = ConfigCapability<Configs>>
        + GetByTag<HttpTag, HttpIdx, Value = std::sync::Arc<HttpCapability<Routes>>>
        + GetByTag<SlotTag, SlotIdx, Value = crate::components::SharedChromeFolder>
        + ProvideRequestCaps
        + Clone
        + Send
        + Sync
        + 'static,
    Configs: GetByTag<AppConfigTag, AppCfgIdx, Value = AppConfig> + Send + Sync,
    Routes: MountRoutes + Clone + Send + Sync,
    CfgIdx: Send + Sync + 'static,
    AppCfgIdx: Send + Sync + 'static,
    HttpIdx: Send + Sync + 'static,
    SlotIdx: Send + Sync + 'static,
{
    type Args = ServeArgs;
    const NAME: &'static str = "serve";
    const ABOUT: &'static str = "Start the HTTP server";

    async fn run(_args: Self::Args, app: MountedApp<M>) -> anyhow::Result<()> {
        app.serve().await
    }
}

/// Attach the command capability with built-in migrate, seed, and serve subcommands.
///
/// # Examples
///
/// ```rust ignore
/// let app = with_commands(app);
/// // After mount:
/// let cli = app.get_capability_output::<CommandTag, _>().build_cli::<_, _>();
/// ```
pub fn with_commands<L, Proof>(app: App<L>) -> App<HCons<CommandCap<HNil, DefaultCommands>, L>>
where
    L: HList + CapTagAbsent<CommandTag, Proof>,
{
    app.add_capability(CapStore::with_items(
        CommandCapability::new()
            .prepend::<MigrateCommandTag, _>(MigrateCommand)
            .prepend::<MarkMigrationsCommandTag, _>(MarkMigrationsCommand)
            .prepend::<SeedCommandTag, _>(SeedCommand)
            .prepend::<ServeCommandTag, _>(ServeCommand)
            .commands,
    ))
}
