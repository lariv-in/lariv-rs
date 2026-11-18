//! Sync the Razorpay IFSC directory once a day while the server is running.

use lariv_core::app::MountedApp;
use lariv_core::hooks::RunServeStartup;
use lariv_core::traits::get::GetByTag;

use crate::sync::{self, RETRY_AFTER_FAILURE};
use crate::{BankAccountsTag, state::BankAccountsState};

/// Start the daily bank-directory syncer (not on migrate or seed).
#[derive(Clone, Copy, Default)]
pub struct ServeStartupHook;

#[async_trait::async_trait]
impl<M, Idx> RunServeStartup<M, Idx> for ServeStartupHook
where
    M: GetByTag<BankAccountsTag, Idx, Value = BankAccountsState> + Sync,
{
    async fn run_serve_startup(app: &MountedApp<M>) -> anyhow::Result<()> {
        let db = app
            .get_capability_output::<BankAccountsTag, Idx>()
            .db
            .clone();
        tokio::spawn(async move {
            loop {
                let last = sync::last_success_at(&db).await;
                let pause = sync::sleep_until_next_sync(last, chrono::Utc::now());
                if !pause.is_zero() {
                    tokio::time::sleep(pause).await;
                }
                match sync::sync_directory(&db).await {
                    Ok(count) => {
                        tracing::info!(
                            target: "bank_directory_sync",
                            count,
                            "bank directory sync complete"
                        );
                    }
                    Err(err) => {
                        tracing::error!(
                            target: "bank_directory_sync",
                            "bank directory sync failed: {err}"
                        );
                        tokio::time::sleep(RETRY_AFTER_FAILURE).await;
                    }
                }
            }
        });
        Ok(())
    }
}
