//! Rune sandbox bindings for customer lookup and mutation.

use lariv_core::rune_env::{RuneEnvCapability, RuneEnvRegistrar};

/// Registers customer helpers onto the assistant Rune environment.
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
        "create_customer",
        "create_customer(#{ company_id?: int, individual_id?: int, state?: string }) -> int  // new or existing customer id. Exactly one of company_id or individual_id is set. state is an optional GST state code such as \"27\"; blank clears it, and omitting it leaves an existing customer's state unchanged.",
        |_ctx| NativeBinding::Function(Arc::new(create_customer)),
    );
    rune_env.register_contextual(
        "update_customer",
        "update_customer(#{ id: int, company_id?: int, individual_id?: int, state?: string }) -> int  // updated customer id. Exactly one of company_id or individual_id is set. state is an optional GST state code; blank clears it, and omitting it keeps the stored state.",
        |_ctx| NativeBinding::Function(Arc::new(update_customer)),
    );
    rune_env.register_contextual(
        "get_customer",
        "get_customer(#{ id: int }) -> #{ id: int, name: string, is_individual: bool, company_id: int|null, individual_id: int|null, state: string|null }",
        |_ctx| NativeBinding::Function(Arc::new(get_customer)),
    );
    rune_env.register_contextual(
        "search_customers",
        "search_customers(#{ query?: string, limit?: int }) -> #{ results: [#{ id: int, name: string }] }",
        |_ctx| NativeBinding::Function(Arc::new(search_customers)),
    );
}

#[cfg(feature = "cap-llm")]
fn create_customer(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let party = parse_party(args, "create_customer")?;
    let db = ctx.db.clone();
    let id = lariv_core::rune_env::block_on_async(async move {
        crate::logic::create_for_party(&db, party.company_id, party.individual_id, party.state)
            .await
    })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn update_customer(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (id, party) = parse_party_update(args)?;
    let db = ctx.db.clone();
    let id = lariv_core::rune_env::block_on_async(async move {
        crate::logic::update_for_party(&db, id, party.company_id, party.individual_id, party.state)
            .await
    })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn get_customer(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = parse_id(args, "get_customer")?;
    let db = ctx.db.clone();
    let row =
        lariv_core::rune_env::block_on_async(
            async move { crate::logic::load_customer(&db, id).await },
        )?;
    lariv_core::rune_env::json_to_rune(serde_json::json!({
        "id": row.id,
        "name": row.name,
        "is_individual": row.is_individual,
        "company_id": row.company_id,
        "individual_id": row.individual_id,
        "state": row.state,
    }))
}

#[cfg(feature = "cap-llm")]
fn search_customers(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (query, limit) = parse_search(args)?;
    let db = ctx.db.clone();
    let rows = lariv_core::rune_env::block_on_async(async move {
        let (rows, _) = crate::logic::search(&db, &query, limit, 0).await?;
        Ok::<_, String>(rows)
    })?;
    lariv_core::rune_env::json_to_rune(serde_json::json!({
        "results": rows.into_iter().map(|row| serde_json::json!({
            "id": row.id,
            "name": row.name,
        })).collect::<Vec<_>>(),
    }))
}

#[cfg(feature = "cap-llm")]
struct PartyWrite {
    company_id: i64,
    individual_id: i64,
    state: crate::logic::CustomerStateUpdate,
}

#[cfg(feature = "cap-llm")]
fn parse_party(args: &[rune::Value], name: &str) -> Result<PartyWrite, String> {
    let parsed = parse_json_value(args, name)?;
    let company_id = json_i64(parsed.get("company_id"));
    let individual_id = json_i64(parsed.get("individual_id"));
    let company = company_id > 0;
    let individual = individual_id > 0;
    if company == individual {
        return Err("select a company or a contact".to_string());
    }
    Ok(PartyWrite {
        company_id,
        individual_id,
        state: state_update(&parsed)?,
    })
}

#[cfg(feature = "cap-llm")]
fn parse_party_update(args: &[rune::Value]) -> Result<(i64, PartyWrite), String> {
    let parsed = parse_json_value(args, "update_customer")?;
    let id = json_i64(parsed.get("id"));
    if id <= 0 {
        return Err("update_customer requires a positive customer id".to_string());
    }
    let party = parse_party(args, "update_customer")?;
    Ok((id, party))
}

#[cfg(feature = "cap-llm")]
fn state_update(value: &serde_json::Value) -> Result<crate::logic::CustomerStateUpdate, String> {
    match value.get("state") {
        None | Some(serde_json::Value::Null) => Ok(crate::logic::CustomerStateUpdate::Keep),
        Some(serde_json::Value::String(raw)) => Ok(crate::logic::CustomerStateUpdate::Set(
            crate::gst_state::parse_state(raw)?,
        )),
        Some(_) => Err("state must be a string".to_string()),
    }
}

#[cfg(feature = "cap-llm")]
fn json_i64(value: Option<&serde_json::Value>) -> i64 {
    match value {
        Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(0),
        Some(serde_json::Value::String(raw)) => raw.parse().unwrap_or(0),
        _ => 0,
    }
}

#[cfg(feature = "cap-llm")]
fn parse_id(args: &[rune::Value], name: &str) -> Result<i64, String> {
    let parsed = parse_json_value(args, name)?;
    let id = json_i64(parsed.get("id"));
    if id <= 0 {
        return Err(format!("{name} requires a positive customer id"));
    }
    Ok(id)
}

#[cfg(feature = "cap-llm")]
fn parse_search(args: &[rune::Value]) -> Result<(String, u64), String> {
    #[derive(serde::Deserialize, Default)]
    struct SearchArgs {
        #[serde(default)]
        query: String,
        #[serde(default)]
        limit: Option<i64>,
    }
    let parsed: SearchArgs = if args.is_empty() {
        SearchArgs::default()
    } else {
        let json = parse_json_value(args, "search_customers")?;
        serde_json::from_value(json)
            .map_err(|e| format!("invalid search_customers arguments: {e}"))?
    };
    let limit = match parsed.limit {
        Some(n) if n > 0 => u64::try_from(n).unwrap_or(50).min(50),
        _ => 20,
    };
    Ok((parsed.query, limit))
}

#[cfg(feature = "cap-llm")]
fn parse_json_value(args: &[rune::Value], name: &str) -> Result<serde_json::Value, String> {
    let value = args
        .first()
        .ok_or_else(|| format!("{name} requires an object argument"))?;
    lariv_core::rune_env::rune_to_json(value).map_err(|e| format!("invalid {name} arguments: {e}"))
}

#[cfg(all(test, feature = "cap-llm"))]
mod tests {
    use super::*;
    use std::sync::Arc;

    use lariv_core::rune_env::RuneEnvCtx;
    use lariv_plugin_llm_assistant::rune_engine;

    fn test_env_ctx(db: &sea_orm::DatabaseConnection) -> RuneEnvCtx<'_> {
        RuneEnvCtx {
            db,
            store: Arc::new(()) as Arc<dyn std::any::Any + Send + Sync>,
            session_id: None,
        }
    }

    fn registered_env() -> RuneEnvCapability {
        let mut cap = RuneEnvCapability::new();
        Hook.register_rune_env(&mut cap);
        cap
    }

    #[test]
    fn registers_customer_bindings() {
        let names = registered_env().all_names();
        for expected in [
            "create_customer",
            "update_customer",
            "get_customer",
            "search_customers",
        ] {
            assert!(
                names.iter().any(|name| name == expected),
                "expected {expected} in {names:?}"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_customer_rejects_neither_party() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let env_ctx = test_env_ctx(&db);
        let out =
            rune_engine::compile_and_run(&cap, &env_ctx, r#"create_customer(#{ })"#, &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("select a company or a contact"),
            "unexpected error payload: {out}"
        );
    }
}
