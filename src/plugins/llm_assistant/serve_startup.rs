//! Start the IMAP email listener when the HTTP server starts (`serve` only).

use crate::{
    app::MountedApp,
    hooks::RunServeStartup,
    traits::get::GetByTag,
};

use super::{LlmAssistantTag, state::LlmAssistantState};

/// Start the background IMAP IDLE listener (not on migrate/seed).
#[derive(Clone, Copy, Default)]
pub struct ServeStartupHook;

#[async_trait::async_trait]
impl<M, AsstIdx> RunServeStartup<M, AsstIdx> for ServeStartupHook
where
    M: GetByTag<LlmAssistantTag, AsstIdx, Value = LlmAssistantState> + Sync,
{
    async fn run_serve_startup(app: &MountedApp<M>) -> anyhow::Result<()> {
        let state = app.get_capability_output::<LlmAssistantTag, AsstIdx>();
        state.email_listener.ensure_started();
        Ok(())
    }
}
