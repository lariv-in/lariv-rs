//! Rune sandbox bindings for company lookup and mutation.

use lariv_core::rune_env::{RuneEnvCapability, RuneEnvRegistrar};

/// Registers company helpers onto the assistant Rune environment.
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
        "create_company",
        "create_company(#{ name: string, address_line_1?: string, address_line_2?: string, city?: string, pincode?: string, state?: string, website?: string, gstin?: string, cin?: string, pan?: string, phone?: string, email?: string }) -> int  // new company id. name is required. state is a GST code such as \"27\" or a state name such as \"Maharashtra\"; blank clears it.",
        |_ctx| NativeBinding::Function(Arc::new(create_company)),
    );
    rune_env.register_contextual(
        "update_company",
        "update_company(#{ id: int, name: string, address_line_1?: string, address_line_2?: string, city?: string, pincode?: string, state?: string, website?: string, gstin?: string, cin?: string, pan?: string, phone?: string, email?: string }) -> int  // updated company id (full replace). state is a GST code or state name; blank clears it.",
        |_ctx| NativeBinding::Function(Arc::new(update_company)),
    );
    rune_env.register_contextual(
        "get_company",
        "get_company(#{ id: int }) -> #{ id: int, name: string, address_line_1: string|null, address_line_2: string|null, city: string|null, pincode: string|null, state: string|null, website: string|null, gstin: string|null, cin: string|null, pan: string|null, phone: string|null, email: string|null }",
        |_ctx| NativeBinding::Function(Arc::new(get_company)),
    );
    rune_env.register_contextual(
        "search_companies",
        "search_companies(#{ query?: string, limit?: int }) -> #{ results: [#{ id: int, name: string, city: string|null, email: string|null, phone: string|null }] }",
        |_ctx| NativeBinding::Function(Arc::new(search_companies)),
    );
}

#[cfg(feature = "cap-llm")]
fn create_company(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let input = parse_company_args(args, "create_company", false)?;
    let owner_user_id = lariv_core::db::db_actor_resolved()
        .ok_or_else(|| "create_company requires a signed-in user".to_string())?;
    let db = ctx.db.clone();
    let id = lariv_core::rune_env::block_on_async(async move {
        let saved = crate::logic::create_company(&db, input, owner_user_id).await?;
        crate::hooks::run_after_create(db, saved.id, owner_user_id).await?;
        Ok::<i64, String>(saved.id)
    })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn update_company(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (id, input) = parse_company_update(args)?;
    let db = ctx.db.clone();
    let id = lariv_core::rune_env::block_on_async(async move {
        let saved = crate::logic::update_company(&db, id, input).await?;
        Ok::<i64, String>(saved.id)
    })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn get_company(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = parse_id(args, "get_company", "company")?;
    let db = ctx.db.clone();
    let row =
        lariv_core::rune_env::block_on_async(
            async move { crate::logic::get_company(&db, id).await },
        )?;
    lariv_core::rune_env::json_to_rune(company_json(&row))
}

#[cfg(feature = "cap-llm")]
fn search_companies(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (query, limit) = parse_search(args, "search_companies")?;
    let db = ctx.db.clone();
    let rows = lariv_core::rune_env::block_on_async(async move {
        crate::logic::search_companies(&db, &query, limit).await
    })?;
    lariv_core::rune_env::json_to_rune(serde_json::json!({
        "results": rows.iter().map(company_summary).collect::<Vec<_>>(),
    }))
}

#[cfg(feature = "cap-llm")]
fn company_json(row: &crate::entities::company::Model) -> serde_json::Value {
    serde_json::json!({
        "id": row.id,
        "name": row.name,
        "address_line_1": row.address_line_1,
        "address_line_2": row.address_line_2,
        "city": row.city,
        "pincode": row.pincode,
        "state": row.state,
        "website": row.website,
        "gstin": row.gstin,
        "cin": row.cin,
        "pan": row.pan,
        "phone": row.phone,
        "email": row.email,
    })
}

#[cfg(feature = "cap-llm")]
fn company_summary(row: &crate::entities::company::Model) -> serde_json::Value {
    serde_json::json!({
        "id": row.id,
        "name": row.name,
        "city": row.city,
        "email": row.email,
        "phone": row.phone,
    })
}

#[cfg(feature = "cap-llm")]
fn parse_company_update(args: &[rune::Value]) -> Result<(i64, crate::logic::CompanyWrite), String> {
    let parsed: CompanyArgs = parse_json(args, "update_company")?;
    let id = positive_id(parsed.id, "update_company", "company")?;
    Ok((id, company_write(parsed)?))
}

#[cfg(feature = "cap-llm")]
fn parse_company_args(
    args: &[rune::Value],
    name: &str,
    _update: bool,
) -> Result<crate::logic::CompanyWrite, String> {
    let parsed: CompanyArgs = parse_json(args, name)?;
    company_write(parsed)
}

#[cfg(feature = "cap-llm")]
fn company_write(parsed: CompanyArgs) -> Result<crate::logic::CompanyWrite, String> {
    Ok(crate::logic::CompanyWrite {
        name: crate::logic::require_name(&parsed.name)?,
        address_line_1: blank(parsed.address_line_1),
        address_line_2: blank(parsed.address_line_2),
        city: blank(parsed.city),
        pincode: blank(parsed.pincode),
        state: crate::gst_state::parse_state(parsed.state.as_deref().unwrap_or(""))?,
        website: blank(parsed.website),
        gstin: blank(parsed.gstin),
        cin: blank(parsed.cin),
        pan: blank(parsed.pan),
        phone: blank(parsed.phone),
        email: blank(parsed.email),
    })
}

#[cfg(feature = "cap-llm")]
#[derive(Debug, serde::Deserialize)]
struct CompanyArgs {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    address_line_1: Option<String>,
    #[serde(default)]
    address_line_2: Option<String>,
    #[serde(default)]
    city: Option<String>,
    #[serde(default)]
    pincode: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    website: Option<String>,
    #[serde(default)]
    gstin: Option<String>,
    #[serde(default)]
    cin: Option<String>,
    #[serde(default)]
    pan: Option<String>,
    #[serde(default)]
    phone: Option<String>,
    #[serde(default)]
    email: Option<String>,
}

#[cfg(feature = "cap-llm")]
fn blank(value: Option<String>) -> Option<String> {
    value.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

#[cfg(feature = "cap-llm")]
fn parse_id(args: &[rune::Value], name: &str, noun: &str) -> Result<i64, String> {
    #[derive(serde::Deserialize)]
    struct IdArgs {
        #[serde(default)]
        id: i64,
    }
    let parsed: IdArgs = parse_json(args, name)?;
    positive_id(parsed.id, name, noun)
}

#[cfg(feature = "cap-llm")]
fn positive_id(id: i64, name: &str, noun: &str) -> Result<i64, String> {
    if id <= 0 {
        Err(format!("{name} requires a positive {noun} id"))
    } else {
        Ok(id)
    }
}

#[cfg(feature = "cap-llm")]
fn parse_search(args: &[rune::Value], name: &str) -> Result<(String, u64), String> {
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
        parse_json(args, name)?
    };
    Ok((parsed.query, clamp_limit(parsed.limit)))
}

#[cfg(feature = "cap-llm")]
fn clamp_limit(limit: Option<i64>) -> u64 {
    match limit {
        Some(n) if n > 0 => u64::try_from(n).unwrap_or(50).min(50),
        _ => 20,
    }
}

#[cfg(feature = "cap-llm")]
fn parse_json<T: serde::de::DeserializeOwned>(
    args: &[rune::Value],
    name: &str,
) -> Result<T, String> {
    let value = args
        .first()
        .ok_or_else(|| format!("{name} requires an object argument"))?;
    let json = lariv_core::rune_env::rune_to_json(value)
        .map_err(|e| format!("invalid {name} arguments: {e}"))?;
    serde_json::from_value(json).map_err(|e| format!("invalid {name} arguments: {e}"))
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
    fn registers_company_bindings() {
        let names = registered_env().all_names();
        for expected in [
            "create_company",
            "update_company",
            "get_company",
            "search_companies",
        ] {
            assert!(
                names.iter().any(|name| name == expected),
                "expected {expected} in {names:?}"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_company_rejects_empty_name() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let env_ctx = test_env_ctx(&db);
        let out =
            rune_engine::compile_and_run(&cap, &env_ctx, r#"create_company(#{ name: "" })"#, &[])
                .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("name is required") || error.contains("name"),
            "unexpected error payload: {out}"
        );
    }
}
