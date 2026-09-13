//! Interval cron jobs — on serve start, run overdue jobs and arm timers for the rest.
//!
//! Each firing creates a new assistant conversation with the job prompt and starts
//! an LLM turn. Last activation is the latest [`super::entities::CronJobRun`] datetime.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};
use tokio::sync::Notify;

use crate::plugins::users::entities::user::{self, Entity as UserEntity};

use super::actions::run_stream_turn;
use super::entities::{
    cron_job::{self, Entity as CronJobEntity},
    cron_job_run::{self, Entity as CronJobRunEntity},
    session,
};
use super::genai::{Content, Role};
use super::live_turn;
use super::preferences::load_preferences;
use super::state::LlmAssistantState;

const LOG_TARGET: &str = "llm_assistant::cron";
/// Re-check at least this often so a long interval still notices clock/job changes.
const MAX_SLEEP: Duration = Duration::from_secs(60 * 60);
const MIN_DURATION_NS: i64 = 1_000_000_000;

/// Signals the background scheduler to reload jobs after create/update/delete.
#[derive(Clone)]
pub struct CronSchedulerHandle {
    state: Arc<OnceLock<Arc<LlmAssistantState>>>,
    reload: Arc<Notify>,
    started: Arc<AtomicBool>,
    running: Arc<Mutex<HashSet<i64>>>,
}

impl CronSchedulerHandle {
    /// Register the shared assistant state used by the background task.
    pub fn bind(&self, state: Arc<LlmAssistantState>) {
        let _ = self.state.set(state);
    }

    /// Ensure the background scheduler is running (idempotent).
    pub fn ensure_started(&self) {
        if self
            .started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }

        let state_slot = Arc::clone(&self.state);
        let reload = self.reload.clone();
        let running = self.running.clone();
        tokio::spawn(async move {
            tracing::info!(target: LOG_TARGET, "cron scheduler started");
            loop {
                let wait = reload.notified();
                tokio::pin!(wait);
                let Some(state) = state_slot.get().cloned() else {
                    tracing::error!(target: LOG_TARGET, "cron scheduler started before state bind");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                };
                let sleep_for = schedule_pass(&state, &running, &reload).await;
                tokio::select! {
                    () = wait => {}
                    () = tokio::time::sleep(sleep_for) => {}
                }
            }
        });
    }

    /// Wake the scheduler so it reloads jobs (after CRUD).
    pub fn reload(&self) {
        self.ensure_started();
        self.reload.notify_waiters();
    }

    /// Start a job immediately (manual run). Records the run before returning so
    /// the conversation appears in history; the LLM turn continues in the background.
    ///
    /// If this job is already running, this is a no-op.
    pub async fn run_now(
        &self,
        state: &LlmAssistantState,
        job: cron_job::Model,
    ) -> anyhow::Result<()> {
        self.ensure_started();
        if !mark_running(&self.running, job.id) {
            return Ok(());
        }
        let reload = self.reload.clone();
        match start_job_run(state, &reload, &job).await {
            Ok(session_id) => {
                let state = state.clone();
                let running = self.running.clone();
                tokio::spawn(async move {
                    if let Err(e) = run_llm_turn(&state, session_id, &job.prompt).await {
                        tracing::error!(
                            target: LOG_TARGET,
                            job_id = job.id,
                            "cron job failed: {e:#}"
                        );
                        tokio::time::sleep(Duration::from_secs(30)).await;
                    }
                    unmark_running(&running, job.id);
                    reload.notify_waiters();
                });
                Ok(())
            }
            Err(e) => {
                unmark_running(&self.running, job.id);
                Err(e)
            }
        }
    }
}

/// Create a handle; call [`CronSchedulerHandle::bind`] then [`CronSchedulerHandle::ensure_started`].
pub fn new_handle() -> CronSchedulerHandle {
    CronSchedulerHandle {
        state: Arc::new(OnceLock::new()),
        reload: Arc::new(Notify::new()),
        started: Arc::new(AtomicBool::new(false)),
        running: Arc::new(Mutex::new(HashSet::new())),
    }
}

/// Next fire time: last run (or created_at if never run) plus the interval.
pub fn next_run_at(
    created_at: DateTime<Utc>,
    last_run: Option<DateTime<Utc>>,
    duration_ns: i64,
) -> Option<DateTime<Utc>> {
    if duration_ns <= 0 {
        return None;
    }
    let anchor = last_run.unwrap_or(created_at);
    let delta = chrono::Duration::nanoseconds(duration_ns);
    anchor.checked_add_signed(delta)
}

async fn schedule_pass(
    state: &LlmAssistantState,
    running: &Arc<Mutex<HashSet<i64>>>,
    reload: &Arc<Notify>,
) -> Duration {
    let jobs = match CronJobEntity::find()
        .order_by_asc(cron_job::Column::Id)
        .all(&state.db)
        .await
    {
        Ok(jobs) => jobs,
        Err(e) => {
            tracing::error!(target: LOG_TARGET, "failed to load cron jobs: {e:#}");
            return Duration::from_secs(30);
        }
    };

    if jobs.is_empty() {
        return MAX_SLEEP;
    }

    let now = Utc::now();
    let mut next_wait: Option<Duration> = None;

    for job in jobs {
        let last = match latest_run_datetime(&state.db, job.id).await {
            Ok(dt) => dt,
            Err(e) => {
                tracing::error!(target: LOG_TARGET, job_id = job.id, "failed to load last run: {e:#}");
                continue;
            }
        };
        let created = job.created_at.unwrap_or(now);
        let Some(due_at) = next_run_at(created, last, job.duration) else {
            tracing::warn!(target: LOG_TARGET, job_id = job.id, "cron job has invalid duration");
            continue;
        };
        if due_at <= now {
            if !is_running(running, job.id) {
                spawn_job_run(state, running, reload, job);
            }
            continue;
        }
        if let Ok(wait) = (due_at - now).to_std() {
            next_wait = Some(match next_wait {
                Some(cur) => cur.min(wait),
                None => wait,
            });
        }
    }

    next_wait.unwrap_or(MAX_SLEEP).min(MAX_SLEEP)
}

fn is_running(running: &Arc<Mutex<HashSet<i64>>>, job_id: i64) -> bool {
    running
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .contains(&job_id)
}

fn mark_running(running: &Arc<Mutex<HashSet<i64>>>, job_id: i64) -> bool {
    running
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(job_id)
}

fn unmark_running(running: &Arc<Mutex<HashSet<i64>>>, job_id: i64) {
    running
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&job_id);
}

async fn latest_run_datetime(
    db: &DatabaseConnection,
    cron_job_id: i64,
) -> Result<Option<DateTime<Utc>>, sea_orm::DbErr> {
    let run = CronJobRunEntity::find()
        .filter(cron_job_run::Column::CronJobId.eq(cron_job_id))
        .order_by_desc(cron_job_run::Column::Datetime)
        .one(db)
        .await?;
    Ok(run.map(|r| r.datetime))
}

fn spawn_job_run(
    state: &LlmAssistantState,
    running: &Arc<Mutex<HashSet<i64>>>,
    reload: &Arc<Notify>,
    job: cron_job::Model,
) {
    if !mark_running(running, job.id) {
        return;
    }
    let state = state.clone();
    let running = running.clone();
    let reload = reload.clone();
    tokio::spawn(async move {
        if let Err(e) = run_job(&state, &reload, &job).await {
            tracing::error!(target: LOG_TARGET, job_id = job.id, "cron job failed: {e:#}");
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
        unmark_running(&running, job.id);
        reload.notify_waiters();
    });
}

async fn run_job(
    state: &LlmAssistantState,
    reload: &Notify,
    job: &cron_job::Model,
) -> anyhow::Result<()> {
    let session_id = start_job_run(state, reload, job).await?;
    run_llm_turn(state, session_id, &job.prompt).await
}

async fn start_job_run(
    state: &LlmAssistantState,
    reload: &Notify,
    job: &cron_job::Model,
) -> anyhow::Result<i64> {
    let owner_id = resolve_session_owner(&state.db).await?;
    let now = Utc::now();
    let title = session_title_from_prompt(job.id, &job.prompt);

    let session_model = session::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        title: Set(title),
        user_id: Set(owner_id),
        reply_email: Set(None),
        email_message_id: Set(None),
        email_references: Set(None),
        context_tokens: Set(0),
        is_subagent: Set(false),
    };
    let session = session_model.insert(&state.db).await?;
    let session_id = session.id;

    let run_model = cron_job_run::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        cron_job_id: Set(job.id),
        datetime: Set(now),
        session_id: Set(Some(session_id)),
    };
    run_model.insert(&state.db).await?;
    // Arm the next interval from this activation without waiting for the LLM turn.
    reload.notify_waiters();

    tracing::info!(
        target: LOG_TARGET,
        job_id = job.id,
        session_id,
        "running cron job"
    );
    Ok(session_id)
}

async fn run_llm_turn(
    state: &LlmAssistantState,
    session_id: i64,
    prompt: &str,
) -> anyhow::Result<()> {
    let (tx, _rx) = live_turn::new_turn_channel();
    let cancel = tokio_util::sync::CancellationToken::new();
    state
        .live_turns
        .insert(session_id, tx.clone(), cancel.clone());
    let store = Arc::clone(&state.email_automation.store);
    let tools = Arc::clone(&state.email_automation.tools);
    let rune_env = Arc::clone(&state.email_automation.rune_env);
    let user = Content::text(Role::User, prompt.to_string());
    let result = run_stream_turn(
        state, store, tools, rune_env, session_id, user, tx, cancel, None,
    )
    .await;
    state.live_turns.remove(session_id);
    result.map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

fn session_title_from_prompt(job_id: i64, prompt: &str) -> String {
    let collapsed: String = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return format!("Cron job #{job_id}");
    }
    const MAX: usize = 72;
    if collapsed.chars().count() <= MAX {
        collapsed
    } else {
        let mut truncated: String = collapsed.chars().take(MAX.saturating_sub(1)).collect();
        truncated.push('…');
        truncated
    }
}

async fn resolve_session_owner(db: &DatabaseConnection) -> anyhow::Result<i64> {
    let prefs = load_preferences(db).await?;
    if let Some(id) = prefs.email_owner_user_id.filter(|id| *id > 0) {
        if UserEntity::find_by_id(id).one(db).await?.is_some() {
            return Ok(id);
        }
        tracing::warn!(
            target: LOG_TARGET,
            user_id = id,
            "configured session owner not found; falling back to first superuser"
        );
    }
    let superuser = UserEntity::find()
        .filter(user::Column::IsSuperuser.eq(true))
        .order_by_asc(user::Column::Id)
        .one(db)
        .await?;
    superuser
        .map(|u| u.id)
        .ok_or_else(|| anyhow::anyhow!("no session owner: set Preferences → Session owner"))
}

/// Parse a duration form value; requires at least one second.
pub fn parse_job_duration(s: &str) -> Result<i64, String> {
    let nanos = crate::duration::parse_duration(s)?;
    if nanos < MIN_DURATION_NS {
        return Err("duration must be at least 1 second".to_string());
    }
    Ok(nanos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn next_run_uses_created_at_when_never_run() {
        let created = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let hour = 3_600 * 1_000_000_000;
        let next = next_run_at(created, None, hour).unwrap();
        assert_eq!(next, created + chrono::Duration::hours(1));
    }

    #[test]
    fn next_run_uses_last_activation() {
        let created = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let last = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
        let hour = 3_600 * 1_000_000_000;
        let next = next_run_at(created, Some(last), hour).unwrap();
        assert_eq!(next, last + chrono::Duration::hours(1));
    }

    #[test]
    fn next_run_rejects_non_positive_duration() {
        let created = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        assert!(next_run_at(created, None, 0).is_none());
        assert!(next_run_at(created, None, -1).is_none());
    }
}
