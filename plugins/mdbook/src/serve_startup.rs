//! Publish the book saved in preferences when `serve` starts.

use lariv_core::app::MountedApp;
use lariv_core::hooks::RunServeStartup;
use lariv_core::traits::get::GetByTag;

use super::preferences::load_preferences;
use super::publish::{PublishOutcome, publish};
use super::{MdBookTag, state::MdBookState};

/// Rebuild the book before the HTTP server listens.
#[derive(Clone, Copy, Default)]
pub struct ServeStartupHook;

#[async_trait::async_trait]
impl<M, Idx> RunServeStartup<M, Idx> for ServeStartupHook
where
    M: GetByTag<MdBookTag, Idx, Value = MdBookState> + Sync,
{
    async fn run_serve_startup(app: &MountedApp<M>) -> anyhow::Result<()> {
        let state = app.get_capability_output::<MdBookTag, Idx>();
        let prefs = load_preferences(&state.db, state.store.as_ref()).await?;
        if !prefs.build_on_serve {
            return Ok(());
        }
        match publish(&state.db, state.store.as_ref(), &prefs).await? {
            PublishOutcome::MissingSource => {
                tracing::info!(
                    source_vnode_id = prefs.source_vnode_id,
                    "mdbook: source directory not found; skipping build"
                );
            }
            PublishOutcome::MissingSummary { source, relative } => {
                tracing::info!(
                    %source,
                    %relative,
                    "mdbook: source directory is not a book yet; skipping build"
                );
            }
            PublishOutcome::Published { files, prefix } => {
                tracing::info!(files, %prefix, "mdbook: site ready");
            }
        }
        Ok(())
    }
}
