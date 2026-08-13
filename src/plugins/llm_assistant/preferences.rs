//! Singleton LLM Assistant preferences (`id = 1`).

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, DbErr, EntityTrait};

use super::entities::{
    LlmAssistantPreferences,
    llm_assistant_preferences::{self, Entity as PrefsEntity},
};

/// Load singleton preferences row (`id = 1`), creating it if missing.
pub async fn load_preferences(db: &DatabaseConnection) -> Result<LlmAssistantPreferences, DbErr> {
    if let Some(prefs) = PrefsEntity::find_by_id(1).one(db).await? {
        return Ok(prefs);
    }

    let now = Utc::now();
    let model = llm_assistant_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        api_key: Set(String::new()),
    };
    model.insert(db).await
}

/// Persist preferences fields onto the singleton row.
pub async fn save_preferences(
    db: &DatabaseConnection,
    prefs: LlmAssistantPreferences,
) -> Result<LlmAssistantPreferences, DbErr> {
    let mut am: llm_assistant_preferences::ActiveModel = load_preferences(db).await?.into();
    am.api_key = Set(prefs.api_key);
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await
}

/// Gemini API key: preferences row first, then `GOOGLE_API_KEY` / `GEMINI_API_KEY`.
pub async fn resolved_api_key(db: &DatabaseConnection) -> Result<String, DbErr> {
    let prefs = load_preferences(db).await?;
    let key = prefs.api_key.trim();
    if !key.is_empty() {
        return Ok(key.to_string());
    }
    Ok(std::env::var("GOOGLE_API_KEY")
        .or_else(|_| std::env::var("GEMINI_API_KEY"))
        .unwrap_or_default())
}
