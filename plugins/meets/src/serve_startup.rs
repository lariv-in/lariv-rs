//! Start the embedded MoQ relay when the HTTP server starts (`serve` only).

use std::sync::Arc;

use lariv_core::app::MountedApp;
use lariv_core::hooks::RunServeStartup;
use lariv_core::traits::get::GetByTag;

use super::{MeetsTag, state::MeetsState};
use lariv_plugin_users::{UsersTag, state::UsersState};

/// Spawn the meets MoQ relay accept loop.
#[derive(Clone, Copy, Default)]
pub struct ServeStartupHook;

#[async_trait::async_trait]
impl<M, MeetsIdx, UsersIdx> RunServeStartup<M, (MeetsIdx, UsersIdx)> for ServeStartupHook
where
    M: GetByTag<MeetsTag, MeetsIdx, Value = MeetsState> + Sync,
    M: GetByTag<UsersTag, UsersIdx, Value = UsersState> + Sync,
{
    async fn run_serve_startup(app: &MountedApp<M>) -> anyhow::Result<()> {
        let meets = app.get_capability_output::<MeetsTag, MeetsIdx>();
        super::relay::start(Arc::new(meets.clone())).await?;
        Ok(())
    }
}
