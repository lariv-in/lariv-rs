//! Background child sessions spawned from LLM tools.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use futures_util::future::try_join_all;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde_json::{Value, json};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::{
    genai::util::content_answer_text,
    llm_tools::{HitlGate, SubagentHost},
    plugins::filesystem::storage::DynFilestore,
};

use super::{
    actions::run_stream_turn,
    content::load_session_turns,
    entities::session::{self, Entity as SessionEntity},
    genai::{Content, Role},
    live_turn::{self, StreamEvent},
    state::LlmAssistantState,
};

const LOG_TARGET: &str = "llm_assistant::subagents";
const TITLE_MAX: usize = 72;
const WAIT_RETRY: Duration = Duration::from_millis(25);

/// [`SubagentHost`] backed by assistant state and the parent turn's HITL gate.
#[derive(Clone)]
pub struct SubagentHostImpl {
    state: LlmAssistantState,
    hitl_gate: Option<HitlGate>,
}

impl SubagentHostImpl {
    pub fn new(state: LlmAssistantState, hitl_gate: Option<HitlGate>) -> Self {
        Self { state, hitl_gate }
    }

    async fn caller_user_id(&self, caller_session_id: i64) -> Result<i64, String> {
        let sess = SessionEntity::find_by_id(caller_session_id)
            .one(&self.state.db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "session not found".to_string())?;
        Ok(sess.user_id)
    }

    async fn load_child(
        &self,
        caller_session_id: i64,
        session_id: i64,
    ) -> Result<session::Model, String> {
        let caller_user_id = self.caller_user_id(caller_session_id).await?;
        let child = SessionEntity::find_by_id(session_id)
            .one(&self.state.db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "subagent session not found".to_string())?;
        if !child.is_subagent {
            return Err("session is not a subagent".into());
        }
        if child.user_id != caller_user_id {
            return Err("subagent belongs to another user".into());
        }
        Ok(child)
    }

    async fn snapshot(&self, child: &session::Model) -> Result<Value, String> {
        let running = self.state.live_turns.contains(child.id);
        let turns = load_session_turns(&self.state.db, child.id)
            .await
            .map_err(|e| e.to_string())?;
        let last_text = turns
            .iter()
            .rev()
            .find(|t| t.content.role == Role::Model)
            .map(|t| content_answer_text(&t.content))
            .unwrap_or_default();
        Ok(json!({
            "session_id": child.id,
            "status": if running { "running" } else { "completed" },
            "title": child.title,
            "last_text": last_text.clone(),
            "result": last_text,
        }))
    }

    async fn wait_one(&self, caller_session_id: i64, session_id: i64) -> Result<Value, String> {
        loop {
            let child = self.load_child(caller_session_id, session_id).await?;
            if !self.state.live_turns.contains(child.id) {
                return self.snapshot(&child).await;
            }
            match self.state.live_turns.subscribe(child.id) {
                Some(mut rx) => loop {
                    match rx.recv().await {
                        Ok(StreamEvent::TurnReady | StreamEvent::Stopped) => break,
                        Ok(_) => {}
                        Err(broadcast::error::RecvError::Lagged(_)) => {}
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                },
                None => tokio::time::sleep(WAIT_RETRY).await,
            }
        }
    }
}

/// Returns the latest model response text (result) of a subagent session.
pub async fn subagent_result(
    db: &sea_orm::DatabaseConnection,
    session_id: i64,
) -> Result<String, String> {
    let sess = SessionEntity::find_by_id(session_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "subagent session not found".to_string())?;
    if !sess.is_subagent {
        return Err("session is not a subagent".into());
    }
    let turns = load_session_turns(db, session_id)
        .await
        .map_err(|e| e.to_string())?;
    let last_text = turns
        .iter()
        .rev()
        .find(|t| t.content.role == Role::Model)
        .map(|t| content_answer_text(&t.content))
        .unwrap_or_default();
    Ok(last_text)
}

#[async_trait]
impl SubagentHost for SubagentHostImpl {
    async fn spawn_subagent(
        &self,
        parent_session_id: i64,
        prompt: String,
        title: Option<String>,
    ) -> Result<Value, String> {
        let prompt = prompt.trim();
        if prompt.is_empty() {
            return Err("prompt is required".into());
        }
        let user_id = self.caller_user_id(parent_session_id).await?;
        let now = Utc::now();
        let title = title
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| title_from_prompt(prompt));

        let saved = session::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            title: Set(title),
            user_id: Set(user_id),
            reply_email: Set(None),
            email_message_id: Set(None),
            email_references: Set(None),
            context_tokens: Set(0),
            is_subagent: Set(true),
        }
        .insert(&self.state.db)
        .await
        .map_err(|e| e.to_string())?;
        let session_id = saved.id;

        let (tx, _rx) = live_turn::new_turn_channel();
        let cancel = CancellationToken::new();
        self.state
            .live_turns
            .insert(session_id, tx.clone(), cancel.clone());

        let state = self.state.clone();
        let store: Arc<DynFilestore> = Arc::clone(&state.email_automation.store);
        let tools = Arc::clone(&state.email_automation.tools);
        let rune_env = Arc::clone(&state.email_automation.rune_env);
        let live_turns = state.live_turns.clone();
        let hitl_gate = self.hitl_gate.clone();
        let user = Content::text(Role::User, prompt.to_string());

        tokio::spawn(async move {
            let result = run_stream_turn(
                &state, store, tools, rune_env, session_id, user, tx, cancel, hitl_gate,
            )
            .await;
            live_turns.remove(session_id);
            if let Err(e) = result {
                tracing::error!(
                    target: LOG_TARGET,
                    session_id,
                    "subagent turn failed: {e}"
                );
            }
        });

        Ok(json!({
            "session_id": session_id,
            "status": "running",
        }))
    }

    async fn subagent_status(
        &self,
        caller_session_id: i64,
        session_id: i64,
    ) -> Result<Value, String> {
        let child = self.load_child(caller_session_id, session_id).await?;
        self.snapshot(&child).await
    }

    async fn subagent_result(
        &self,
        caller_session_id: i64,
        session_id: i64,
    ) -> Result<Value, String> {
        let child = self.load_child(caller_session_id, session_id).await?;
        self.snapshot(&child).await
    }

    async fn wait_subagents(
        &self,
        caller_session_id: i64,
        session_ids: Vec<i64>,
    ) -> Result<Value, String> {
        if session_ids.is_empty() {
            return Err("session_ids is required".into());
        }
        let futs = session_ids.into_iter().map(|id| {
            let host = self.clone();
            async move { host.wait_one(caller_session_id, id).await }
        });
        let subagents = try_join_all(futs).await?;
        Ok(json!({ "subagents": subagents }))
    }
}

fn title_from_prompt(prompt: &str) -> String {
    let collapsed: String = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return "Subagent".into();
    }
    if collapsed.chars().count() <= TITLE_MAX {
        collapsed
    } else {
        let mut truncated: String = collapsed
            .chars()
            .take(TITLE_MAX.saturating_sub(1))
            .collect();
        truncated.push('…');
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, ConnectionTrait, Database, Schema, Statement};

    use crate::plugins::llm_assistant::{
        content::save_content,
        entities::{part_text, session, session_message, session_message_part, video_metadata},
        genai::{Content, Role},
    };

    async fn setup_test_db() -> sea_orm::DatabaseConnection {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("sqlite memory");
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);

        db.execute_raw(Statement::from_string(
            backend,
            "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT)".to_string(),
        ))
        .await
        .expect("users");
        db.execute_raw(Statement::from_string(
            backend,
            "INSERT INTO users (id) VALUES (1)".to_string(),
        ))
        .await
        .expect("user row");

        for stmt in [
            schema.create_table_from_entity(session::Entity),
            schema.create_table_from_entity(session_message::Entity),
            schema.create_table_from_entity(video_metadata::Entity),
            schema.create_table_from_entity(session_message_part::Entity),
            schema.create_table_from_entity(part_text::Entity),
        ] {
            db.execute(&stmt).await.expect("create table");
        }
        db
    }

    #[test]
    fn title_from_prompt_collapses_whitespace() {
        assert_eq!(title_from_prompt("  hello   world  "), "hello world");
    }

    #[test]
    fn title_from_prompt_truncates() {
        let long = "word ".repeat(40);
        let title = title_from_prompt(&long);
        assert!(title.chars().count() <= 72);
        assert!(title.ends_with('…'));
    }

    #[tokio::test]
    async fn subagent_result_returns_last_model_text() {
        let db = setup_test_db().await;
        let now = Utc::now();
        let child = session::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            title: Set("Child task".to_string()),
            user_id: Set(1),
            reply_email: Set(None),
            email_message_id: Set(None),
            email_references: Set(None),
            context_tokens: Set(0),
            is_subagent: Set(true),
        }
        .insert(&db)
        .await
        .unwrap();

        save_content(
            &db,
            child.id,
            &Content::text(Role::User, "Calculate 2 + 2".to_string()),
        )
        .await
        .unwrap();
        save_content(
            &db,
            child.id,
            &Content::text(Role::Model, "The answer is 4.".to_string()),
        )
        .await
        .unwrap();

        let result = subagent_result(&db, child.id).await.unwrap();
        assert_eq!(result, "The answer is 4.");
    }

    #[tokio::test]
    async fn subagent_result_rejects_non_subagent() {
        let db = setup_test_db().await;
        let now = Utc::now();
        let normal_sess = session::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            title: Set("Main session".to_string()),
            user_id: Set(1),
            reply_email: Set(None),
            email_message_id: Set(None),
            email_references: Set(None),
            context_tokens: Set(0),
            is_subagent: Set(false),
        }
        .insert(&db)
        .await
        .unwrap();

        let err = subagent_result(&db, normal_sess.id).await.unwrap_err();
        assert!(err.contains("not a subagent"), "{err}");
    }

    #[tokio::test]
    async fn subagent_result_missing_session() {
        let db = setup_test_db().await;
        let err = subagent_result(&db, 99999).await.unwrap_err();
        assert!(err.contains("not found"), "{err}");
    }
}
