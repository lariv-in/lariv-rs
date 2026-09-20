//! Rune sandbox bindings for form creation.

use crate::rune_env::{RuneEnvCapability, RuneEnvRegistrar};

/// Registers form helpers onto the assistant Rune environment.
#[derive(Clone, Copy, Default)]
pub struct Hook;

impl RuneEnvRegistrar for Hook {
    fn register_rune_env(self, rune_env: &mut RuneEnvCapability) {
        #[cfg(feature = "cap-llm")]
        register(rune_env);
        #[cfg(not(feature = "cap-llm"))]
        let _ = rune_env;
    }
}

#[cfg(feature = "cap-llm")]
fn register(rune_env: &mut RuneEnvCapability) {
    use std::sync::Arc;

    use crate::rune_env::NativeBinding;

    rune_env.register_contextual(
        "create_form",
        "create_form(#{ title: string, questions?: [#{ display_text: string, question_type: string|object, required?: bool, description?: string|null, form_question_id?: string }], created_by_id?: int }) -> int  // new form id",
        |_ctx| NativeBinding::Function(Arc::new(create_form)),
    );
}

#[cfg(feature = "cap-llm")]
fn create_form(
    ctx: &crate::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let input = parse_create_args(args)?;
    let db = ctx.db.clone();
    let id = crate::rune_env::block_on_async(async move { insert_form(&db, input).await })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
mod args {
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
    use serde::Deserialize;
    use uuid::Uuid;

    use crate::plugins::forms::{
        entities::form,
        logic::questions::parse_questions_json,
        types::{FormQuestion, FormQuestionId, FormQuestionType, FormQuestions},
    };
    use crate::plugins::users::entities::user::Entity as UserEntity;

    #[derive(Debug, Deserialize)]
    pub(super) struct CreateQuestionArg {
        #[serde(default)]
        form_question_id: Option<String>,
        display_text: String,
        question_type: FormQuestionType,
        #[serde(default)]
        required: bool,
        #[serde(default)]
        description: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub(super) struct CreateFormArgs {
        title: String,
        #[serde(default)]
        questions: Vec<CreateQuestionArg>,
        #[serde(default)]
        created_by_id: i64,
    }

    pub(super) struct CreateFormInput {
        pub title: String,
        pub questions: FormQuestions,
        pub created_by_id: i64,
    }

    fn parse_question_id(raw: &str) -> Result<FormQuestionId, String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(FormQuestionId::new());
        }
        Uuid::parse_str(trimmed)
            .map(FormQuestionId::from)
            .map_err(|e| format!("invalid form_question_id: {e}"))
    }

    fn questions_from_args(raw: Vec<CreateQuestionArg>) -> Result<FormQuestions, String> {
        let questions: Vec<FormQuestion> = raw
            .into_iter()
            .map(|q| {
                let form_question_id = match q.form_question_id.as_deref() {
                    Some(id) => parse_question_id(id)?,
                    None => FormQuestionId::new(),
                };
                Ok(FormQuestion {
                    form_question_id,
                    display_text: q.display_text,
                    question_type: q.question_type,
                    required: q.required,
                    description: q.description.filter(|d| !d.trim().is_empty()),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let json = serde_json::to_string(&FormQuestions(questions))
            .map_err(|e| format!("invalid questions: {e}"))?;
        parse_questions_json(&json)
    }

    async fn resolve_created_by_id(
        db: &sea_orm::DatabaseConnection,
        raw: i64,
    ) -> Result<i64, String> {
        if raw > 0 {
            let exists = UserEntity::find_by_id(raw)
                .one(db)
                .await
                .map_err(|e| e.to_string())?
                .is_some();
            if !exists {
                return Err(format!("user {raw} not found"));
            }
            return Ok(raw);
        }
        let user = UserEntity::find()
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "create_form requires created_by_id when no users exist".to_string())?;
        Ok(user.id)
    }

    pub(super) fn parse_create_args(args: &[rune::Value]) -> Result<CreateFormInput, String> {
        let value = args
            .first()
            .ok_or_else(|| "create_form requires an object argument".to_string())?;
        let parsed: CreateFormArgs = serde_json::from_value(
            crate::rune_env::rune_to_json(value)
                .map_err(|e| format!("invalid create_form arguments: {e}"))?,
        )
        .map_err(|e| format!("invalid create_form arguments: {e}"))?;
        let title = parsed.title.trim().to_string();
        if title.is_empty() {
            return Err("create_form requires title".into());
        }
        Ok(CreateFormInput {
            title,
            questions: questions_from_args(parsed.questions)?,
            created_by_id: parsed.created_by_id,
        })
    }

    pub(super) async fn insert_form(
        db: &sea_orm::DatabaseConnection,
        input: CreateFormInput,
    ) -> Result<i64, String> {
        let created_by_id = resolve_created_by_id(db, input.created_by_id).await?;
        let now = Utc::now();
        let model = form::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            title: Set(input.title),
            questions: Set(input.questions),
            created_by_id: Set(created_by_id),
        };
        let saved = model.insert(db).await.map_err(|e| e.to_string())?;
        Ok(saved.id)
    }
}

#[cfg(feature = "cap-llm")]
use args::{insert_form, parse_create_args};

#[cfg(all(test, feature = "cap-llm"))]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::plugins::filesystem::storage::{DynFilestore, UnimplementedFilestore};
    use crate::plugins::llm_assistant::rune_engine;
    use crate::rune_env::{RuneEnvCapability, RuneEnvCtx};

    fn test_env_ctx<'a>(
        db: &'a sea_orm::DatabaseConnection,
        store: &'a Arc<DynFilestore>,
    ) -> RuneEnvCtx<'a> {
        RuneEnvCtx {
            db,
            store: Arc::clone(store),
            session_id: None,
        }
    }

    fn registered_env() -> RuneEnvCapability {
        let mut cap = RuneEnvCapability::new();
        Hook.register_rune_env(&mut cap);
        cap
    }

    #[test]
    fn registers_create_form_binding() {
        let names = registered_env().all_names();
        assert!(
            names.iter().any(|name| name == "create_form"),
            "expected create_form in {names:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_form_via_rune_rejects_missing_title() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_form(#{ questions: [#{ display_text: "Name", question_type: "ShortText" }] })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(error.contains("title"), "unexpected error payload: {out}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_form_via_rune_rejects_empty_title() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out =
            rune_engine::compile_and_run(&cap, &env_ctx, r#"create_form(#{ title: "  " })"#, &[])
                .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("create_form requires title"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_form_via_rune_rejects_missing_argument() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "create_form(())", &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("create_form requires an object argument")
                || error.contains("unsupported"),
            "unexpected error payload: {out}"
        );
    }
}
