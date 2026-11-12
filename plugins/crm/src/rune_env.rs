//! Rune sandbox bindings for leads and lead tags.

use lariv_core::rune_env::{RuneEnvCapability, RuneEnvRegistrar};

#[cfg(feature = "cap-llm")]
use sea_orm::EntityTrait;

/// Registers lead and lead-tag helpers onto the assistant Rune environment.
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
        "create_lead",
        "create_lead(#{ customer_id: int, source?: string, notes?: string, assigned_to_id?: int, order_expected_date?: string, tag_ids?: [int], created_by_id?: int }) -> int  // new lead id. customer_id is required. source is web_form, cold_outreach, scrape_list, referral, or other. Writes a lead update. created_by_id is the update author when the script is not running inside an assistant session.",
        |_ctx| NativeBinding::Function(Arc::new(create_lead)),
    );
    rune_env.register_contextual(
        "update_lead",
        "update_lead(#{ id: int, customer_id: int, source?: string, notes?: string, assigned_to_id?: int, order_expected_date?: string, tag_ids?: [int], created_by_id?: int }) -> int  // updated lead id. Omitted tag_ids keeps the current tags; a present tag_ids replaces them. Writes a lead update.",
        |_ctx| NativeBinding::Function(Arc::new(update_lead)),
    );
    rune_env.register_contextual(
        "get_lead",
        "get_lead(#{ id: int }) -> #{ id: int, customer_id: int, status: string, source: string|null, notes: string|null, assigned_to_id: int|null, order_expected_date: string|null, tag_ids: [int], updated_at: string|null }  // status is active, converted, or failed",
        |_ctx| NativeBinding::Function(Arc::new(get_lead)),
    );
    rune_env.register_contextual(
        "search_leads",
        "search_leads(#{ query?: string, limit?: int, status?: string }) -> #{ results: [#{ id: int, customer_id: int, status: string, source: string|null, notes: string|null, assigned_to_id: int|null, order_expected_date: string|null }] }  // status is active, converted, or failed",
        |_ctx| NativeBinding::Function(Arc::new(search_leads)),
    );
    rune_env.register_contextual(
        "search_active_leads",
        "search_active_leads(#{ query?: string, limit?: int }) -> #{ results: [#{ id: int, customer_id: int, status: string }] }",
        |_ctx| NativeBinding::Function(Arc::new(search_active_leads)),
    );
    rune_env.register_contextual(
        "search_converted_leads",
        "search_converted_leads(#{ query?: string, limit?: int }) -> #{ results: [#{ id: int, customer_id: int, status: string }] }",
        |_ctx| NativeBinding::Function(Arc::new(search_converted_leads)),
    );
    rune_env.register_contextual(
        "search_failed_leads",
        "search_failed_leads(#{ query?: string, limit?: int }) -> #{ results: [#{ id: int, customer_id: int, status: string }] }",
        |_ctx| NativeBinding::Function(Arc::new(search_failed_leads)),
    );
    rune_env.register_contextual(
        "convert_lead",
        "convert_lead(#{ id: int, created_by_id?: int }) -> int  // converted-lead id. Copies the lead's customer. Writes a lead update.",
        |_ctx| NativeBinding::Function(Arc::new(convert_lead)),
    );
    rune_env.register_contextual(
        "fail_lead",
        "fail_lead(#{ id: int, reason?: string, created_by_id?: int }) -> int  // failed-lead id. Active or converted leads can be failed. Writes a lead update.",
        |_ctx| NativeBinding::Function(Arc::new(fail_lead)),
    );
    rune_env.register_contextual(
        "reactivate_lead",
        "reactivate_lead(#{ id: int, created_by_id?: int }) -> int  // lead id made active again from converted or failed. Writes a lead update.",
        |_ctx| NativeBinding::Function(Arc::new(reactivate_lead)),
    );
    rune_env.register_contextual(
        "create_lead_update",
        "create_lead_update(#{ lead_id: int, description: string, datetime?: string, created_by_id?: int }) -> int  // new lead update id. datetime is RFC3339 or YYYY-MM-DDTHH:MM.",
        |_ctx| NativeBinding::Function(Arc::new(create_lead_update)),
    );
    rune_env.register_contextual(
        "update_lead_update",
        "update_lead_update(#{ id: int, lead_id: int, description: string, datetime?: string, created_by_id?: int }) -> int",
        |_ctx| NativeBinding::Function(Arc::new(update_lead_update)),
    );
    rune_env.register_contextual(
        "get_lead_update",
        "get_lead_update(#{ id: int }) -> #{ id: int, lead_id: int, created_by_id: int, datetime: string, description: string }",
        |_ctx| NativeBinding::Function(Arc::new(get_lead_update)),
    );
    rune_env.register_contextual(
        "search_lead_updates",
        "search_lead_updates(#{ lead_id?: int, query?: string, limit?: int }) -> #{ results: [#{ id: int, lead_id: int, created_by_id: int, datetime: string, description: string }] }",
        |_ctx| NativeBinding::Function(Arc::new(search_lead_updates)),
    );
    rune_env.register_contextual(
        "create_lead_tag",
        "create_lead_tag(#{ name: string, color?: string }) -> int  // new lead tag id. color is #rrggbb; omitted color uses the default.",
        |_ctx| NativeBinding::Function(Arc::new(create_lead_tag)),
    );
    rune_env.register_contextual(
        "update_lead_tag",
        "update_lead_tag(#{ id: int, name: string, color?: string }) -> int  // updated lead tag id",
        |_ctx| NativeBinding::Function(Arc::new(update_lead_tag)),
    );
    rune_env.register_contextual(
        "get_lead_tag",
        "get_lead_tag(#{ id: int }) -> #{ id: int, name: string, color: string }",
        |_ctx| NativeBinding::Function(Arc::new(get_lead_tag)),
    );
    rune_env.register_contextual(
        "search_lead_tags",
        "search_lead_tags(#{ query?: string, limit?: int }) -> #{ results: [#{ id: int, name: string, color: string }] }",
        |_ctx| NativeBinding::Function(Arc::new(search_lead_tags)),
    );
}

#[cfg(feature = "cap-llm")]
fn create_lead(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let write = parse_lead(args, "create_lead", false)?;
    let session_id = ctx.session_id;
    let db = ctx.db.clone();
    let id = lariv_core::rune_env::block_on_async(async move {
        save_lead(&db, session_id, None, write, "Lead created").await
    })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn update_lead(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let write = parse_lead(args, "update_lead", true)?;
    let lead_id = write.id;
    let session_id = ctx.session_id;
    let db = ctx.db.clone();
    let id = lariv_core::rune_env::block_on_async(async move {
        save_lead(&db, session_id, Some(lead_id), write, "Lead updated").await
    })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn get_lead(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = parse_id(args, "get_lead", "lead")?;
    let db = ctx.db.clone();
    let row = lariv_core::rune_env::block_on_async(async move { load_lead(&db, id).await })?;
    lariv_core::rune_env::json_to_rune(row)
}

#[cfg(feature = "cap-llm")]
fn search_leads(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (query, limit, status) = parse_lead_search(args)?;
    let db = ctx.db.clone();
    let rows = lariv_core::rune_env::block_on_async(async move {
        search_lead_rows(&db, &query, limit, status.as_deref()).await
    })?;
    lariv_core::rune_env::json_to_rune(serde_json::json!({ "results": rows }))
}

#[cfg(feature = "cap-llm")]
fn create_lead_tag(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (name, color) = parse_tag(args, "create_lead_tag", false)?;
    let db = ctx.db.clone();
    let id = lariv_core::rune_env::block_on_async(async move {
        let saved = crate::logic::lead_tag::create_lead_tag(&db, &name, &color).await?;
        Ok::<i64, String>(saved.id)
    })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn update_lead_tag(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = parse_id(args, "update_lead_tag", "lead tag")?;
    let (name, color) = parse_tag(args, "update_lead_tag", true)?;
    let db = ctx.db.clone();
    let id = lariv_core::rune_env::block_on_async(async move {
        let saved = crate::logic::lead_tag::update_lead_tag(&db, id, &name, &color).await?;
        Ok::<i64, String>(saved.id)
    })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn get_lead_tag(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = parse_id(args, "get_lead_tag", "lead tag")?;
    let db = ctx.db.clone();
    let row = lariv_core::rune_env::block_on_async(async move {
        crate::logic::lead_tag::get_lead_tag(&db, id).await
    })?;
    lariv_core::rune_env::json_to_rune(tag_json(&row))
}

#[cfg(feature = "cap-llm")]
fn search_lead_tags(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let (query, limit) = parse_search(args, "search_lead_tags")?;
    let db = ctx.db.clone();
    let rows = lariv_core::rune_env::block_on_async(async move {
        crate::logic::lead_tag::search_lead_tags(&db, &query, limit).await
    })?;
    lariv_core::rune_env::json_to_rune(serde_json::json!({
        "results": rows.iter().map(tag_json).collect::<Vec<_>>(),
    }))
}

#[cfg(feature = "cap-llm")]
fn tag_json(row: &crate::entities::lead_tag::Model) -> serde_json::Value {
    serde_json::json!({
        "id": row.id,
        "name": row.name,
        "color": row.color,
    })
}

#[cfg(feature = "cap-llm")]
struct LeadWrite {
    id: i64,
    customer_id: i64,
    source: Option<crate::lead_source::LeadSource>,
    notes: Option<String>,
    assigned_to_id: Option<i64>,
    order_expected_date: Option<chrono::NaiveDate>,
    tag_ids: Option<Vec<i64>>,
    created_by_id: Option<i64>,
}

#[cfg(feature = "cap-llm")]
async fn save_lead(
    db: &sea_orm::DatabaseConnection,
    session_id: Option<i64>,
    lead_id: Option<i64>,
    write: LeadWrite,
    description: &'static str,
) -> Result<i64, String> {
    use sea_orm::TransactionTrait;

    db.transaction(|txn| {
        let write = write_parts(&write);
        Box::pin(async move {
            let author = resolve_author(txn, session_id, write.created_by_id).await?;
            ensure_customer(txn, write.customer_id).await?;
            if let Some(user_id) = write.assigned_to_id {
                ensure_user(txn, user_id).await?;
            }
            let tag_ids = match write.tag_ids {
                Some(ids) => {
                    ensure_tags(txn, &ids).await?;
                    ids
                }
                None => match lead_id {
                    Some(id) => crate::logic::lead::tag_ids_for_lead(txn, id).await?,
                    None => Vec::new(),
                },
            };
            let input = crate::logic::lead::LeadInput {
                customer_id: write.customer_id,
                source: write.source,
                notes: write.notes,
                assigned_to_id: write.assigned_to_id,
                order_expected_date: write.order_expected_date,
                tag_ids,
            };
            let saved = match lead_id {
                Some(id) => crate::logic::lead::update_lead(txn, id, input).await?,
                None => crate::logic::lead::create_lead(txn, input).await?,
            };
            crate::logic::lead_update::append_lead_update(txn, saved.id, author, description)
                .await?;
            Ok::<i64, String>(saved.id)
        })
    })
    .await
    .map_err(tx_err)
}

#[cfg(feature = "cap-llm")]
struct LeadParts {
    customer_id: i64,
    source: Option<crate::lead_source::LeadSource>,
    notes: Option<String>,
    assigned_to_id: Option<i64>,
    order_expected_date: Option<chrono::NaiveDate>,
    tag_ids: Option<Vec<i64>>,
    created_by_id: Option<i64>,
}

#[cfg(feature = "cap-llm")]
fn write_parts(write: &LeadWrite) -> LeadParts {
    LeadParts {
        customer_id: write.customer_id,
        source: write.source,
        notes: write.notes.clone(),
        assigned_to_id: write.assigned_to_id,
        order_expected_date: write.order_expected_date,
        tag_ids: write.tag_ids.clone(),
        created_by_id: write.created_by_id,
    }
}

#[cfg(feature = "cap-llm")]
fn tx_err(err: sea_orm::TransactionError<String>) -> String {
    match err {
        sea_orm::TransactionError::Connection(err) => err.to_string(),
        sea_orm::TransactionError::Transaction(err) => err,
    }
}

#[cfg(feature = "cap-llm")]
async fn resolve_author<C: sea_orm::ConnectionTrait>(
    db: &C,
    session_id: Option<i64>,
    explicit: Option<i64>,
) -> Result<i64, String> {
    if let Some(session_id) = session_id.filter(|id| *id > 0) {
        let session = lariv_plugin_llm_assistant::entities::session::Entity::find_by_id(session_id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("assistant session {session_id} not found"))?;
        ensure_user(db, session.user_id).await?;
        return Ok(session.user_id);
    }
    let Some(id) = explicit.filter(|id| *id > 0) else {
        return Err("created_by_id is required when no assistant session is active".to_string());
    };
    ensure_user(db, id).await?;
    Ok(id)
}

#[cfg(feature = "cap-llm")]
async fn ensure_user<C: sea_orm::ConnectionTrait>(db: &C, user_id: i64) -> Result<(), String> {
    let found = lariv_plugin_users::entities::user::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    if found.is_none() {
        return Err(format!("user {user_id} not found"));
    }
    Ok(())
}

#[cfg(feature = "cap-llm")]
async fn ensure_customer<C: sea_orm::ConnectionTrait>(
    db: &C,
    customer_id: i64,
) -> Result<(), String> {
    let found = lariv_plugin_customer::entities::customer::Entity::find_by_id(customer_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    if found.is_none() {
        return Err("customer not found".to_string());
    }
    Ok(())
}

#[cfg(feature = "cap-llm")]
async fn ensure_tags<C: sea_orm::ConnectionTrait>(db: &C, tag_ids: &[i64]) -> Result<(), String> {
    for tag_id in tag_ids.iter().copied().filter(|id| *id > 0) {
        let found = crate::entities::lead_tag::Entity::find_by_id(tag_id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?;
        if found.is_none() {
            return Err(format!("lead tag {tag_id} not found"));
        }
    }
    Ok(())
}

#[cfg(feature = "cap-llm")]
async fn load_lead<C: sea_orm::ConnectionTrait>(
    db: &C,
    lead_id: i64,
) -> Result<serde_json::Value, String> {
    let lead = crate::entities::lead::Entity::find_by_id(lead_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("lead {lead_id} not found"))?;
    let tag_ids = crate::logic::lead::tag_ids_for_lead(db, lead.id).await?;
    let status = crate::logic::lead_update::lead_status(db, lead.id).await?;
    Ok(lead_json(&lead, Some(tag_ids), Some(status)))
}

#[cfg(feature = "cap-llm")]
async fn search_lead_rows<C: sea_orm::ConnectionTrait>(
    db: &C,
    query: &str,
    limit: u64,
    status: Option<&str>,
) -> Result<Vec<serde_json::Value>, String> {
    use lariv_plugin_contacts::entities::{company, contact};
    use lariv_plugin_customer::entities::customer;
    use sea_orm::sea_query::{Condition, Expr, ExprTrait, Func, Query};
    use sea_orm::{
        ColumnTrait, EntityTrait, JoinType, QueryFilter, QueryOrder, QuerySelect, RelationTrait,
    };

    let mut select = crate::entities::lead::Entity::find()
        .join(
            JoinType::LeftJoin,
            crate::entities::lead::Relation::Customer.def(),
        )
        .join(JoinType::LeftJoin, customer::Relation::Individual.def())
        .join(JoinType::LeftJoin, customer::Relation::Company.def());
    if let Some(status) = status {
        let converted_ids = Query::select()
            .column(crate::entities::converted_lead::Column::LeadId)
            .from(crate::entities::converted_lead::Entity)
            .to_owned();
        let failed_ids = Query::select()
            .column(crate::entities::failed_lead::Column::LeadId)
            .from(crate::entities::failed_lead::Entity)
            .to_owned();
        select = match status {
            "active" => select.filter(crate::scope::lead_active_condition()),
            "converted" => {
                select.filter(crate::entities::lead::Column::Id.in_subquery(converted_ids))
            }
            "failed" => select.filter(crate::entities::lead::Column::Id.in_subquery(failed_ids)),
            _ => return Err("status must be active, converted, or failed".to_string()),
        };
    }
    let needle = query.trim();
    if !needle.is_empty() {
        let like = format!(
            "%{}%",
            needle
                .to_lowercase()
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        select = select.filter(
            Condition::any()
                .add(
                    Func::lower(Expr::col((contact::Entity, contact::Column::Name)))
                        .like(like.clone()),
                )
                .add(
                    Func::lower(Expr::col((company::Entity, company::Column::Name)))
                        .like(like.clone()),
                )
                .add(Func::lower(Expr::col(crate::entities::lead::Column::Notes)).like(like)),
        );
    }
    let rows = select
        .order_by_desc(crate::entities::lead::Column::Id)
        .limit(limit)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let status = crate::logic::lead_update::lead_status(db, row.id).await?;
        out.push(lead_json(&row, None, Some(status)));
    }
    Ok(out)
}

#[cfg(feature = "cap-llm")]
fn lead_json(
    lead: &crate::entities::lead::Model,
    tag_ids: Option<Vec<i64>>,
    status: Option<&str>,
) -> serde_json::Value {
    let mut value = serde_json::json!({
        "id": lead.id,
        "customer_id": lead.customer_id,
        "source": lead.source.map(|source| source.as_str()),
        "notes": lead.notes,
        "assigned_to_id": lead.assigned_to_id,
        "order_expected_date": lead.order_expected_date.map(|date| date.format("%Y-%m-%d").to_string()),
        "updated_at": lead.updated_at.map(|stamp| stamp.to_rfc3339()),
    });
    if let Some(tag_ids) = tag_ids
        && let Some(object) = value.as_object_mut()
    {
        object.insert("tag_ids".to_string(), serde_json::json!(tag_ids));
    }
    if let Some(status) = status
        && let Some(object) = value.as_object_mut()
    {
        object.insert("status".to_string(), serde_json::json!(status));
    }
    value
}

#[cfg(feature = "cap-llm")]
#[derive(Debug, serde::Deserialize)]
struct LeadArgs {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    customer_id: i64,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    assigned_to_id: i64,
    #[serde(default)]
    order_expected_date: Option<String>,
    #[serde(default)]
    tag_ids: Option<Vec<i64>>,
    #[serde(default)]
    created_by_id: Option<i64>,
}

#[cfg(feature = "cap-llm")]
fn parse_lead(args: &[rune::Value], name: &str, update: bool) -> Result<LeadWrite, String> {
    let parsed: LeadArgs = parse_json(args, name)?;
    if update && parsed.id <= 0 {
        return Err(format!("{name} requires a positive lead id"));
    }
    if parsed.customer_id <= 0 {
        return Err("customer is required".to_string());
    }
    let source = match parsed.source.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(raw) => Some(
            crate::lead_source::LeadSource::parse(raw)
                .ok_or_else(|| format!("invalid lead source {raw}"))?,
        ),
    };
    let order_expected_date = match parsed.order_expected_date.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(raw) => Some(
            lariv_core::datetime::parse_date(raw)
                .ok_or_else(|| "invalid order expected date".to_string())?,
        ),
    };
    let notes = parsed.notes.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    Ok(LeadWrite {
        id: parsed.id,
        customer_id: parsed.customer_id,
        source,
        notes,
        assigned_to_id: (parsed.assigned_to_id > 0).then_some(parsed.assigned_to_id),
        order_expected_date,
        tag_ids: parsed.tag_ids,
        created_by_id: parsed.created_by_id.filter(|id| *id > 0),
    })
}

#[cfg(feature = "cap-llm")]
fn parse_tag(args: &[rune::Value], name: &str, update: bool) -> Result<(String, String), String> {
    #[derive(serde::Deserialize)]
    struct TagArgs {
        #[serde(default)]
        name: String,
        #[serde(default)]
        color: String,
    }
    let parsed: TagArgs = parse_json(args, name)?;
    if update {
        let _ = parse_id(args, name, "lead tag")?;
    }
    let name_value = parsed.name.trim();
    if name_value.is_empty() {
        return Err("name is required".to_string());
    }
    let color = if parsed.color.trim().is_empty() {
        crate::logic::lead_tag::DEFAULT_TAG_COLOR.to_string()
    } else {
        crate::logic::lead_tag::parsed_tag_color(&parsed.color)
            .ok_or_else(|| "invalid color".to_string())?
    };
    Ok((name_value.to_string(), color))
}

#[cfg(feature = "cap-llm")]
fn parse_id(args: &[rune::Value], name: &str, noun: &str) -> Result<i64, String> {
    #[derive(serde::Deserialize)]
    struct IdArgs {
        #[serde(default)]
        id: i64,
    }
    let parsed: IdArgs = parse_json(args, name)?;
    if parsed.id <= 0 {
        return Err(format!("{name} requires a positive {noun} id"));
    }
    Ok(parsed.id)
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
    let limit = match parsed.limit {
        Some(n) if n > 0 => u64::try_from(n).unwrap_or(50).min(50),
        _ => 20,
    };
    Ok((parsed.query, limit))
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

#[cfg(feature = "cap-llm")]
fn search_active_leads(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    search_leads_with_status(ctx, args, "search_active_leads", "active")
}

#[cfg(feature = "cap-llm")]
fn search_converted_leads(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    search_leads_with_status(ctx, args, "search_converted_leads", "converted")
}

#[cfg(feature = "cap-llm")]
fn search_failed_leads(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    search_leads_with_status(ctx, args, "search_failed_leads", "failed")
}

#[cfg(feature = "cap-llm")]
fn search_leads_with_status(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
    name: &str,
    status: &'static str,
) -> Result<rune::Value, String> {
    let (query, limit) = parse_search(args, name)?;
    let db = ctx.db.clone();
    let rows = lariv_core::rune_env::block_on_async(async move {
        search_lead_rows(&db, &query, limit, Some(status)).await
    })?;
    lariv_core::rune_env::json_to_rune(serde_json::json!({ "results": rows }))
}

#[cfg(feature = "cap-llm")]
fn convert_lead(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = parse_id(args, "convert_lead", "lead")?;
    let created_by_id = optional_author(args, "convert_lead")?;
    switch_lead(
        ctx,
        id,
        created_by_id,
        "Lead converted",
        Switch::Convert { id },
    )
}

#[cfg(feature = "cap-llm")]
fn fail_lead(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = parse_id(args, "fail_lead", "lead")?;
    let created_by_id = optional_author(args, "fail_lead")?;
    let reason = parse_reason(args)?;
    let description = match reason.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(reason) => format!("Lead marked failed: {reason}"),
        None => "Lead marked failed".to_string(),
    };
    switch_lead(
        ctx,
        id,
        created_by_id,
        description,
        Switch::Fail { id, reason },
    )
}

#[cfg(feature = "cap-llm")]
fn reactivate_lead(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = parse_id(args, "reactivate_lead", "lead")?;
    let created_by_id = optional_author(args, "reactivate_lead")?;
    switch_lead(
        ctx,
        id,
        created_by_id,
        "Lead reactivated".to_string(),
        Switch::Reactivate { id },
    )
}

#[cfg(feature = "cap-llm")]
enum Switch {
    Convert { id: i64 },
    Fail { id: i64, reason: Option<String> },
    Reactivate { id: i64 },
}

#[cfg(feature = "cap-llm")]
fn switch_lead(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    lead_id: i64,
    created_by_id: Option<i64>,
    description: impl Into<String>,
    action: Switch,
) -> Result<rune::Value, String> {
    let description = description.into();
    let session_id = ctx.session_id;
    let db = ctx.db.clone();
    let id = lariv_core::rune_env::block_on_async(async move {
        use sea_orm::TransactionTrait;
        db.transaction(|txn| {
            let description = description.clone();
            Box::pin(async move {
                let author = resolve_author(txn, session_id, created_by_id).await?;
                let result_id = match action {
                    Switch::Convert { id } => {
                        crate::logic::lead_conversion::convert_active_lead(txn, id)
                            .await?
                            .converted_id
                    }
                    Switch::Fail { id, reason } => {
                        crate::logic::lead_fail::fail_lead_unscoped(txn, id, reason).await?
                    }
                    Switch::Reactivate { id } => {
                        crate::logic::lead_fail::make_lead_active(txn, id).await?;
                        id
                    }
                };
                crate::logic::lead_update::append_lead_update(txn, lead_id, author, &description)
                    .await?;
                Ok::<i64, String>(result_id)
            })
        })
        .await
        .map_err(tx_err)
    })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn create_lead_update(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let write = parse_lead_update(args, "create_lead_update", false)?;
    let session_id = ctx.session_id;
    let db = ctx.db.clone();
    let id = lariv_core::rune_env::block_on_async(async move {
        save_lead_update(&db, session_id, None, write).await
    })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn update_lead_update(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let write = parse_lead_update(args, "update_lead_update", true)?;
    let update_id = write.id;
    let session_id = ctx.session_id;
    let db = ctx.db.clone();
    let id = lariv_core::rune_env::block_on_async(async move {
        save_lead_update(&db, session_id, Some(update_id), write).await
    })?;
    Ok(rune::Value::from(id))
}

#[cfg(feature = "cap-llm")]
fn get_lead_update(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = parse_id(args, "get_lead_update", "lead update")?;
    let db = ctx.db.clone();
    let row = lariv_core::rune_env::block_on_async(async move {
        crate::logic::lead_update::get_lead_update(&db, id).await
    })?;
    lariv_core::rune_env::json_to_rune(lead_update_json(&row))
}

#[cfg(feature = "cap-llm")]
fn search_lead_updates(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    #[derive(serde::Deserialize, Default)]
    struct Args {
        #[serde(default)]
        lead_id: i64,
        #[serde(default)]
        query: String,
        #[serde(default)]
        limit: Option<i64>,
    }
    let parsed: Args = if args.is_empty() {
        Args::default()
    } else {
        parse_json(args, "search_lead_updates")?
    };
    let limit = match parsed.limit {
        Some(n) if n > 0 => u64::try_from(n).unwrap_or(50).min(50),
        _ => 20,
    };
    let lead_id = (parsed.lead_id > 0).then_some(parsed.lead_id);
    let query = parsed.query;
    let db = ctx.db.clone();
    let rows = lariv_core::rune_env::block_on_async(async move {
        crate::logic::lead_update::search_lead_updates(&db, lead_id, &query, limit).await
    })?;
    lariv_core::rune_env::json_to_rune(serde_json::json!({
        "results": rows.iter().map(lead_update_json).collect::<Vec<_>>(),
    }))
}

#[cfg(feature = "cap-llm")]
fn lead_update_json(row: &crate::entities::lead_update::Model) -> serde_json::Value {
    serde_json::json!({
        "id": row.id,
        "lead_id": row.lead_id,
        "created_by_id": row.created_by_id,
        "datetime": row.datetime.to_rfc3339(),
        "description": row.description,
    })
}

#[cfg(feature = "cap-llm")]
struct ParsedLeadUpdate {
    id: i64,
    lead_id: i64,
    description: String,
    datetime: chrono::DateTime<chrono::Utc>,
    created_by_id: Option<i64>,
}

#[cfg(feature = "cap-llm")]
fn parse_lead_update(
    args: &[rune::Value],
    name: &str,
    update: bool,
) -> Result<ParsedLeadUpdate, String> {
    #[derive(serde::Deserialize)]
    struct Args {
        #[serde(default)]
        id: i64,
        #[serde(default)]
        lead_id: i64,
        #[serde(default)]
        description: String,
        #[serde(default)]
        datetime: Option<String>,
        #[serde(default)]
        created_by_id: Option<i64>,
    }
    let parsed: Args = parse_json(args, name)?;
    if update && parsed.id <= 0 {
        return Err(format!("{name} requires a positive lead update id"));
    }
    if parsed.lead_id <= 0 {
        return Err("lead not found".to_string());
    }
    let description = parsed.description.trim().to_string();
    if description.is_empty() {
        return Err("description is required".to_string());
    }
    Ok(ParsedLeadUpdate {
        id: parsed.id,
        lead_id: parsed.lead_id,
        description,
        datetime: parse_update_datetime(parsed.datetime)?,
        created_by_id: parsed.created_by_id.filter(|id| *id > 0),
    })
}

#[cfg(feature = "cap-llm")]
fn parse_update_datetime(raw: Option<String>) -> Result<chrono::DateTime<chrono::Utc>, String> {
    let Some(raw) = raw else {
        return Ok(chrono::Utc::now());
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(chrono::Utc::now());
    }
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(raw) {
        return Ok(parsed.with_timezone(&chrono::Utc));
    }
    for fmt in ["%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M"] {
        if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(raw, fmt) {
            return Ok(parsed.and_utc());
        }
    }
    Err("invalid datetime".to_string())
}

#[cfg(feature = "cap-llm")]
async fn save_lead_update(
    db: &sea_orm::DatabaseConnection,
    session_id: Option<i64>,
    update_id: Option<i64>,
    write: ParsedLeadUpdate,
) -> Result<i64, String> {
    use sea_orm::TransactionTrait;
    db.transaction(|txn| {
        Box::pin(async move {
            let author = resolve_author(txn, session_id, write.created_by_id).await?;
            let input = crate::logic::lead_update::LeadUpdateWrite {
                lead_id: write.lead_id,
                created_by_id: author,
                datetime: write.datetime,
                description: write.description,
            };
            let saved = match update_id {
                Some(id) => crate::logic::lead_update::update_lead_update(txn, id, input).await?,
                None => crate::logic::lead_update::create_lead_update(txn, input).await?,
            };
            Ok::<i64, String>(saved.id)
        })
    })
    .await
    .map_err(tx_err)
}

#[cfg(feature = "cap-llm")]
fn optional_author(args: &[rune::Value], name: &str) -> Result<Option<i64>, String> {
    #[derive(serde::Deserialize, Default)]
    struct Args {
        #[serde(default)]
        created_by_id: Option<i64>,
    }
    let parsed: Args = parse_json(args, name)?;
    Ok(parsed.created_by_id.filter(|id| *id > 0))
}

#[cfg(feature = "cap-llm")]
fn parse_reason(args: &[rune::Value]) -> Result<Option<String>, String> {
    #[derive(serde::Deserialize, Default)]
    struct Args {
        #[serde(default)]
        reason: Option<String>,
    }
    let parsed: Args = parse_json(args, "fail_lead")?;
    Ok(parsed.reason)
}

#[cfg(feature = "cap-llm")]
fn parse_lead_search(args: &[rune::Value]) -> Result<(String, u64, Option<String>), String> {
    #[derive(serde::Deserialize, Default)]
    struct Args {
        #[serde(default)]
        query: String,
        #[serde(default)]
        limit: Option<i64>,
        #[serde(default)]
        status: Option<String>,
    }
    let parsed: Args = if args.is_empty() {
        Args::default()
    } else {
        parse_json(args, "search_leads")?
    };
    let status = match parsed.status.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(status @ ("active" | "converted" | "failed")) => Some(status.to_string()),
        Some(_) => {
            return Err("status must be active, converted, or failed".to_string());
        }
    };
    let limit = match parsed.limit {
        Some(n) if n > 0 => u64::try_from(n).unwrap_or(50).min(50),
        _ => 20,
    };
    Ok((parsed.query, limit, status))
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
    fn registers_lead_and_tag_bindings() {
        let names = registered_env().all_names();
        for expected in [
            "create_lead",
            "update_lead",
            "get_lead",
            "search_leads",
            "create_lead_tag",
            "update_lead_tag",
            "get_lead_tag",
            "search_lead_tags",
            "search_active_leads",
            "search_converted_leads",
            "search_failed_leads",
            "convert_lead",
            "fail_lead",
            "reactivate_lead",
            "create_lead_update",
            "update_lead_update",
            "get_lead_update",
            "search_lead_updates",
        ] {
            assert!(
                names.iter().any(|name| name == expected),
                "expected {expected} in {names:?}"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_lead_rejects_missing_contact() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let env_ctx = test_env_ctx(&db);
        let out = rune_engine::compile_and_run(&cap, &env_ctx, r#"create_lead(#{ })"#, &[]).await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("customer is required") || error.contains("customer"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_lead_update_rejects_empty_description() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let env_ctx = test_env_ctx(&db);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"create_lead_update(#{ lead_id: 1, description: "" })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("description is required"),
            "unexpected error payload: {out}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn search_leads_rejects_unknown_status() {
        let cap = registered_env();
        let db = sea_orm::DatabaseConnection::default();
        let env_ctx = test_env_ctx(&db);
        let out = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"search_leads(#{ status: "archived" })"#,
            &[],
        )
        .await;
        let error = out
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        assert!(
            error.contains("status must be active, converted, or failed"),
            "unexpected error payload: {out}"
        );
    }
}
