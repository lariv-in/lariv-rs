//! Start the periodic-task polling worker when the HTTP server starts.

use lariv_core::app::MountedApp;
use lariv_core::hooks::RunServeStartup;
use lariv_core::traits::get::GetByTag;

use super::{TasksTag, state::TasksState};

/// Spawn the periodic task worker (`serve` only, not migrate or seed).
#[derive(Clone, Copy, Default)]
pub struct ServeStartupHook;

#[async_trait::async_trait]
impl<M, TasksIdx> RunServeStartup<M, TasksIdx> for ServeStartupHook
where
    M: GetByTag<TasksTag, TasksIdx, Value = TasksState> + Sync,
{
    async fn run_serve_startup(app: &MountedApp<M>) -> anyhow::Result<()> {
        let state = app.get_capability_output::<TasksTag, TasksIdx>();
        let settings = super::push::ensure_settings(&state.db, &state.push).await;
        super::worker::spawn(state.db.clone(), settings);
        Ok(())
    }
}
