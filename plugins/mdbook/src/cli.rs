//! `mdbook-build` rebuilds the book saved in preferences and publishes it.

use clap::Args;
use frunk::{HCons, hlist::HList};

use lariv_core::app::MountedApp;
use lariv_core::command::{CommandCapability, CommandRegistrar, RunCommand};
use lariv_core::tag::Tagged;
use lariv_core::traits::get::GetByTag;

use super::preferences::load_preferences;
use super::publish::{PublishOutcome, publish};
use super::{MdBookTag, state::MdBookState};

/// Tag for [`BuildCommand`].
pub struct BuildCommandTag;

/// Rebuild the book from preferences and replace its website routes.
#[derive(Clone, Copy, Debug, Default)]
pub struct BuildCommand;

/// Args for [`BuildCommand`] (no flags).
#[derive(Args, Debug, Clone, Default)]
pub struct BuildArgs {}

#[async_trait::async_trait]
impl<M, Idx> RunCommand<M, Idx> for BuildCommand
where
    M: GetByTag<MdBookTag, Idx, Value = MdBookState> + Sync + Send + 'static,
    Idx: Send + Sync + 'static,
{
    type Args = BuildArgs;
    const NAME: &'static str = "mdbook-build";
    const ABOUT: &'static str =
        "Build the mdBook saved in preferences and publish it on the website";

    async fn run(_args: Self::Args, app: MountedApp<M>) -> anyhow::Result<()> {
        let state = app.get_capability_output::<MdBookTag, Idx>().clone();
        let prefs = load_preferences(&state.db, state.store.as_ref()).await?;
        match publish(&state.db, state.store.as_ref(), &prefs).await? {
            PublishOutcome::MissingSource => {
                anyhow::bail!(
                    "mdbook source directory {} was not found",
                    prefs.source_vnode_id
                );
            }
            PublishOutcome::MissingSummary { source, relative } => {
                anyhow::bail!("mdbook source directory \"{source}\" has no {relative}");
            }
            PublishOutcome::Published { files, prefix } => {
                println!("published {files} files at {prefix}");
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl<C> CommandRegistrar<C> for Hook
where
    C: HList + Clone,
{
    type Output = MdBookCommands<C>;

    fn register_commands(self, cap: CommandCapability<C>) -> CommandCapability<Self::Output> {
        cap.prepend::<BuildCommandTag, _>(BuildCommand)
    }
}

/// Commands registered by [`Hook`].
pub type MdBookCommands<C> = HCons<Tagged<BuildCommandTag, BuildCommand>, C>;
