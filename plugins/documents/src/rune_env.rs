//! Rune sandbox bindings for creating and editing identity documents.

use lariv_core::rune_env::{RuneEnvCapability, RuneEnvRegistrar};

/// Registers document helpers onto the assistant Rune environment.
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

    use lariv_core::rune_env::NativeBinding;

    rune_env.register_contextual(
        "create_document",
        "create_document(#{ document_type: \"aadhar_card\", vnode_id: int, aadhar_number: string, name: string, gender: \"male\"|\"female\"|\"transgender\", date_of_birth: string, address: string }) -> int  // new document id; date_of_birth is DD/MM/YYYY; vnode_id is the uploaded card file",
        |_ctx| NativeBinding::Function(Arc::new(create_document)),
    );
    rune_env.register_contextual(
        "update_document",
        "update_document(#{ id: int, document_type: \"aadhar_card\", vnode_id: int, aadhar_number: string, name: string, gender: \"male\"|\"female\"|\"transgender\", date_of_birth: string, address: string }) -> int  // updated document id (full replace; document type cannot change); date_of_birth is DD/MM/YYYY; vnode_id is the uploaded card file",
        |_ctx| NativeBinding::Function(Arc::new(update_document)),
    );
}

#[cfg(feature = "cap-llm")]
fn create_document(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let form = parse_create_args(args)?;
    let db = ctx.db.clone();
    let saved = lariv_core::rune_env::block_on_async(async move {
        crate::logic::create_document(&db, &form).await
    })?;
    Ok(rune::Value::from(saved.id))
}

#[cfg(feature = "cap-llm")]
fn update_document(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (id, form) = parse_update_args(args)?;
    crate::logic::validate_document_form(&form)?;
    let db = ctx.db.clone();
    lariv_core::rune_env::block_on_async(async move {
        use sea_orm::EntityTrait;

        use crate::entities::document::Entity as DocumentEntity;

        let existing = DocumentEntity::find_by_id(id)
            .one(&db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("document {id} not found"))?;
        crate::logic::update_document(&db, &existing, &form).await
    })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn parse_create_args(args: &[rune::Value]) -> Result<crate::forms::DocumentForm, String> {
    Ok(document_form(parse_fields(args, "create_document")?))
}

#[cfg(feature = "cap-llm")]
fn parse_update_args(args: &[rune::Value]) -> Result<(i64, crate::forms::DocumentForm), String> {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct UpdateDocumentArgs {
        id: i64,
        #[serde(flatten)]
        fields: DocumentFields,
    }

    let value = args
        .first()
        .ok_or_else(|| "update_document requires an object argument".to_string())?;
    let parsed: UpdateDocumentArgs = serde_json::from_value(
        lariv_core::rune_env::rune_to_json(value)
            .map_err(|e| format!("invalid update_document arguments: {e}"))?,
    )
    .map_err(|e| format!("invalid update_document arguments: {e}"))?;
    if parsed.id <= 0 {
        return Err("update_document requires a positive document id".into());
    }
    Ok((parsed.id, document_form(parsed.fields)))
}

#[cfg(feature = "cap-llm")]
fn parse_fields(args: &[rune::Value], fn_name: &str) -> Result<DocumentFields, String> {
    let value = args
        .first()
        .ok_or_else(|| format!("{fn_name} requires an object argument"))?;
    serde_json::from_value(
        lariv_core::rune_env::rune_to_json(value)
            .map_err(|e| format!("invalid {fn_name} arguments: {e}"))?,
    )
    .map_err(|e| format!("invalid {fn_name} arguments: {e}"))
}

#[cfg(feature = "cap-llm")]
fn document_form(parsed: DocumentFields) -> crate::forms::DocumentForm {
    use crate::forms::DocumentForm;

    DocumentForm {
        document_type: parsed.document_type,
        vnode_id: parsed.vnode_id,
        aadhar_number: parsed.aadhar_number,
        pan_number: parsed.pan_number,
        passport_number: parsed.passport_number,
        name: parsed.name,
        gender: parsed.gender,
        date_of_birth: parsed.date_of_birth,
        address: parsed.address,
        nationality: parsed.nationality,
        expiry_date: parsed.expiry_date,
        csrf: Default::default(),
    }
}

#[cfg(feature = "cap-llm")]
#[derive(Debug, serde::Deserialize)]
struct DocumentFields {
    document_type: String,
    vnode_id: i64,
    #[serde(default)]
    aadhar_number: String,
    #[serde(default)]
    pan_number: String,
    #[serde(default)]
    passport_number: String,
    name: String,
    #[serde(default)]
    gender: String,
    date_of_birth: String,
    #[serde(default)]
    address: String,
    #[serde(default)]
    nationality: String,
    #[serde(default)]
    expiry_date: String,
}

#[cfg(all(test, feature = "plugin-llm-assistant"))]
mod tests {
    use super::*;
    use std::sync::Arc;

    use lariv_core::rune_env::{RuneEnvCapability, RuneEnvCtx};
    use lariv_plugin_filesystem::storage::{DynFilestore, UnimplementedFilestore};
    use lariv_plugin_llm_assistant::rune_engine;

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

    fn error_text(out: &serde_json::Value) -> String {
        out.get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn registers_create_and_update_document_bindings() {
        let names = registered_env().all_names();
        assert!(
            names.iter().any(|name| name == "create_document"),
            "expected create_document in {names:?}"
        );
        assert!(
            names.iter().any(|name| name == "update_document"),
            "expected update_document in {names:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_document_via_rune_rejects_missing_argument() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "create_document(())", &[]).await;
        let error = error_text(&out);
        assert!(
            error.contains("create_document requires an object argument")
                || error.contains("unsupported"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_document_via_rune_rejects_missing_name() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_document(#{ document_type: "aadhar_card", vnode_id: 1, aadhar_number: "123456789012", gender: "female", date_of_birth: "01/01/1990", address: "1 Road" })"#,
            &[],
        )
        .await;
        let error = error_text(&out);
        assert!(
            error.contains("invalid create_document arguments") || error.contains("name"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_document_via_rune_rejects_empty_name() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_document(#{ document_type: "aadhar_card", vnode_id: 0, aadhar_number: "123456789012", name: "  ", gender: "female", date_of_birth: "01/01/1990", address: "1 Road" })"#,
            &[],
        )
        .await;
        let error = error_text(&out);
        assert!(
            error.contains("Name is required"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_document_via_rune_rejects_bad_aadhar_number() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_document(#{ document_type: "aadhar_card", vnode_id: 0, aadhar_number: "123", name: "Ada", gender: "female", date_of_birth: "01/01/1990", address: "1 Road" })"#,
            &[],
        )
        .await;
        let error = error_text(&out);
        assert!(
            error.contains("Aadhar number must be 12 digits"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_document_via_rune_rejects_unknown_type() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_document(#{ document_type: "voter_id", vnode_id: 0, aadhar_number: "123", name: "Ada", gender: "female", date_of_birth: "01/01/1990", address: "1 Road" })"#,
            &[],
        )
        .await;
        let error = error_text(&out);
        assert!(
            error.contains("Unknown document type"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_document_via_rune_accepts_object_built_from_lets() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"
let holder = "Ada Lovelace";
let gender = "other";
create_document(#{
    document_type: "aadhar_card",
    vnode_id: 1,
    aadhar_number: "1234 5678 9012",
    name: holder,
    gender: gender,
    date_of_birth: "01/01/1990",
    address: "1 Road"
})
"#,
            &[],
        )
        .await;
        let error = error_text(&out);
        assert!(
            error.contains("Choose a gender"),
            "object-from-lets conversion failed: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_document_via_rune_rejects_missing_argument() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, "update_document(())", &[]).await;
        let error = error_text(&out);
        assert!(
            error.contains("update_document requires an object argument")
                || error.contains("unsupported"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_document_via_rune_rejects_missing_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_document(#{ document_type: "aadhar_card", vnode_id: 1, aadhar_number: "123456789012", name: "Ada", gender: "female", date_of_birth: "01/01/1990", address: "1 Road" })"#,
            &[],
        )
        .await;
        let error = error_text(&out);
        assert!(
            error.contains("invalid update_document arguments") || error.contains("id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_document_via_rune_rejects_non_positive_id() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_document(#{ id: 0, document_type: "aadhar_card", vnode_id: 1, aadhar_number: "123456789012", name: "Ada", gender: "female", date_of_birth: "01/01/1990", address: "1 Road" })"#,
            &[],
        )
        .await;
        let error = error_text(&out);
        assert!(
            error.contains("update_document requires a positive document id"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_document_via_rune_rejects_empty_name() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"update_document(#{ id: 1, document_type: "aadhar_card", vnode_id: 1, aadhar_number: "123456789012", name: "  ", gender: "female", date_of_birth: "01/01/1990", address: "1 Road" })"#,
            &[],
        )
        .await;
        let error = error_text(&out);
        assert!(
            error.contains("Name is required"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn update_document_via_rune_accepts_object_built_from_lets() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let env_ctx = test_env_ctx(&db, &store);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"
let holder = "Ada Lovelace";
let gender = "other";
update_document(#{
    id: 7,
    document_type: "aadhar_card",
    vnode_id: 1,
    aadhar_number: "1234 5678 9012",
    name: holder,
    gender: gender,
    date_of_birth: "01/01/1990",
    address: "1 Road"
})
"#,
            &[],
        )
        .await;
        let error = error_text(&out);
        assert!(
            error.contains("Choose a gender"),
            "object-from-lets conversion failed: {out}"
        );
    }
}
