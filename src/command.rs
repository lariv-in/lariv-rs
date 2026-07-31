use clap::{ArgMatches, Args, Command as ClapCommand, FromArgMatches};
use frunk::{HCons, HNil, hlist::HList};

use crate::{
    app::{App, MountedApp},
    capability::{ApplyHooks, CapStore, Capability, FoldRegistrarHooks, apply_registrar_hook, mount_with_hooks},
    components::{FoldSlots, SlotCapability, SlotTag},
    config::{AppConfig, AppConfigTag, ConfigCapability, ConfigTag},
    db::{DbState, DbTag},
    hooks::{FoldSeeds, SeedRunner, SeedsTag},
    http::{HttpCapability, HttpTag, MountRoutes, ProvideRequestCaps},
    migration::{MigrationCapability, MigrationTag, RunMigrations},
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

/// Tag for the built-in [`ServeCommand`].
pub struct ServeCommandTag;

/// Plugin hook for appending commands onto a [`CommandCapability`].
pub trait CommandRegistrar<C>: Sized {
    type Output;
    fn register_commands(self, cap: CommandCapability<C>) -> CommandCapability<Self::Output>;
}

/// Registered CLI subcommand: clap metadata + async runner.
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
    async fn dispatch(
        self,
        name: &str,
        _: &ArgMatches,
        _: MountedApp<M>,
    ) -> anyhow::Result<()> {
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

/// Default command HList from [`with_commands`] (migrate, seed, serve).
pub type DefaultCommands = HCons<
    Tagged<ServeCommandTag, ServeCommand>,
    HCons<
        Tagged<SeedCommandTag, SeedCommand>,
        HCons<Tagged<MigrateCommandTag, MigrateCommand>, HNil>,
    >,
>;

/// Run database migrations.
#[derive(Clone, Copy, Debug, Default)]
pub struct MigrateCommand;

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

/// Run registered seed hooks.
#[derive(Clone, Copy, Debug, Default)]
pub struct SeedCommand;

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

/// Start the HTTP server.
#[derive(Clone, Copy, Debug, Default)]
pub struct ServeCommand;

#[derive(Args, Debug, Clone, Default)]
pub struct ServeArgs {}

#[async_trait::async_trait]
impl<M, CfgIdx, Configs, AppCfgIdx, HttpIdx, Routes, SlotIdx, Slots>
    RunCommand<M, (CfgIdx, Configs, AppCfgIdx, HttpIdx, Routes, SlotIdx, Slots)> for ServeCommand
where
    M: GetByTag<ConfigTag, CfgIdx, Value = ConfigCapability<Configs>>
        + GetByTag<HttpTag, HttpIdx, Value = std::sync::Arc<HttpCapability<Routes>>>
        + GetByTag<SlotTag, SlotIdx, Value = SlotCapability<Slots>>
        + ProvideRequestCaps
        + Clone
        + Send
        + Sync
        + 'static,
    Configs: GetByTag<AppConfigTag, AppCfgIdx, Value = AppConfig> + Send + Sync,
    Routes: MountRoutes + Clone + Send + Sync,
    Slots: FoldSlots + Clone + Send + Sync + 'static,
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

/// Attach the command capability with built-in migrate, seed, and serve.
pub fn with_commands<L, Proof>(
    app: App<L>,
) -> App<HCons<CommandCap<HNil, DefaultCommands>, L>>
where
    L: HList + CapTagAbsent<CommandTag, Proof>,
{
    app.add_capability(CapStore::with_items(
        CommandCapability::new()
            .prepend::<MigrateCommandTag, _>(MigrateCommand)
            .prepend::<SeedCommandTag, _>(SeedCommand)
            .prepend::<ServeCommandTag, _>(ServeCommand)
            .commands,
    ))
}
