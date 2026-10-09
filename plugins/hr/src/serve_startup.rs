//! Evaluate leave calculations once an hour while the server is running.

use lariv_core::app::MountedApp;
use lariv_core::hooks::RunServeStartup;
use lariv_core::traits::get::GetByTag;

use super::logic::leave_calc::{duration_until_next_hour, evaluate_due_leaves};
use super::{HrTag, state::HrState};

/// Start the hourly leave evaluator (not on migrate or seed).
#[derive(Clone, Copy, Default)]
pub struct ServeStartupHook;

#[async_trait::async_trait]
impl<M, HrIdx> RunServeStartup<M, HrIdx> for ServeStartupHook
where
    M: GetByTag<HrTag, HrIdx, Value = HrState> + Sync,
{
    async fn run_serve_startup(app: &MountedApp<M>) -> anyhow::Result<()> {
        let db = app.get_capability_output::<HrTag, HrIdx>().db.clone();
        tokio::spawn(async move {
            loop {
                if let Err(err) = evaluate_due_leaves(&db).await {
                    tracing::error!(target: "hr_leave_calc", "leave evaluation failed: {err}");
                }
                let pause = duration_until_next_hour(chrono::Utc::now());
                tokio::time::sleep(pause).await;
            }
        });
        Ok(())
    }
}
