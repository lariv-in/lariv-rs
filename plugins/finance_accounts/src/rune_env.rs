//! Rune sandbox bindings for reading accounts, journal entries, and source documents.
//!
//! Journal lines store a debit as a positive amount and a credit as a negative
//! amount. Account `balance` is the signed sum of that account and its
//! descendants, so a positive balance is a net debit.

use lariv_core::rune_env::{RuneEnvCapability, RuneEnvRegistrar};

/// Registers ledger read helpers onto the assistant Rune environment.
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
        "get_account",
        "get_account(#{ id: int }) -> #{ id: int, name: string, code: int, is_group: bool, balance_type: \"Debit\"|\"Credit\", parent_id: int|null, balance: string, ancestors: [#{ id: int, name: string, code: int }], children: [#{ id: int, name: string, code: int, is_group: bool, balance_type: \"Debit\"|\"Credit\" }] }  // balance is the signed sum of this account and its descendants. Positive is net debit. ancestors run from the root to the parent. children are the direct children.",
        |_ctx| NativeBinding::Function(Arc::new(get_account)),
    );
    rune_env.register_contextual(
        "search_accounts",
        "search_accounts(#{ query: string, limit?: int, balance_type?: \"Debit\"|\"Credit\", parent_id?: int, leaves_only?: bool }) -> #{ results: [#{ id: int, name: string, code: int, is_group: bool, balance_type: \"Debit\"|\"Credit\", parent_id: int|null, balance: string }] }  // query matches the account name. A numeric query also matches that account code. parent_id limits results to accounts under that account. leaves_only keeps posting accounts.",
        |_ctx| NativeBinding::Function(Arc::new(search_accounts)),
    );
    rune_env.register_contextual(
        "list_accounts",
        "list_accounts(#{ parent_id?: int, balance_type?: \"Debit\"|\"Credit\", leaves_only?: bool, limit?: int }) -> #{ results: [#{ id: int, name: string, code: int, is_group: bool, balance_type: \"Debit\"|\"Credit\", parent_id: int|null, balance: string }] }  // parent_id 0 lists root accounts. A positive parent_id lists its direct children. An omitted parent_id lists the chart.",
        |_ctx| NativeBinding::Function(Arc::new(list_accounts)),
    );
    rune_env.register_contextual(
        "get_journal_entry",
        "get_journal_entry(#{ id: int }) -> #{ id: int, datetime: string, journal_id: int, journal_name: string|null, source_doc_id: int, source_doc_type: string|null, source_ref_id: int|null, transferred_amount: string, lines: [#{ id: int, account_id: int, account_name: string|null, account_code: int|null, amount: string, debit: string|null, credit: string|null }] }  // amount is positive for a debit and negative for a credit. transferred_amount is that debit total. source_doc_id is the source_docs row. source_ref_id is the business document id, such as a posted invoice.",
        |_ctx| NativeBinding::Function(Arc::new(get_journal_entry)),
    );
    rune_env.register_contextual(
        "list_journal_entries",
        "list_journal_entries(#{ journal_id?: int, journal?: string, account_id?: int, account?: string, source_doc_id?: int, source_doc_type?: string, source_ref_id?: int, from?: string, to?: string, limit?: int }) -> #{ results: [#{ id: int, datetime: string, journal_id: int, journal_name: string|null, source_doc_id: int, source_doc_type: string|null, source_ref_id: int|null, transferred_amount: string, lines: [#{ id: int, account_id: int, account_name: string|null, account_code: int|null, amount: string, debit: string|null, credit: string|null }] }] }  // Filters combine. account_id and account include descendant accounts. source_ref_id requires source_doc_type. from and to are inclusive ISO datetimes or calendar dates, interpreted in UTC. journal matches the journal name. account matches the account name, and a numeric account matches the code.",
        |_ctx| NativeBinding::Function(Arc::new(list_journal_entries)),
    );
    rune_env.register_contextual(
        "get_journal_entry_item",
        "get_journal_entry_item(#{ id: int }) -> #{ id: int, datetime: string, journal_entry_id: int, journal_id: int|null, journal_name: string|null, source_doc_id: int|null, source_doc_type: string|null, source_ref_id: int|null, account_id: int, account_name: string|null, account_code: int|null, amount: string, debit: string|null, credit: string|null }  // amount is positive for a debit and negative for a credit.",
        |_ctx| NativeBinding::Function(Arc::new(get_journal_entry_item)),
    );
    rune_env.register_contextual(
        "list_journal_entry_items",
        "list_journal_entry_items(#{ journal_entry_id?: int, journal_id?: int, journal?: string, account_id?: int, account?: string, source_doc_id?: int, source_doc_type?: string, source_ref_id?: int, side?: \"debit\"|\"credit\", from?: string, to?: string, limit?: int }) -> #{ results: [#{ id: int, datetime: string, journal_entry_id: int, journal_id: int|null, journal_name: string|null, source_doc_id: int|null, source_doc_type: string|null, source_ref_id: int|null, account_id: int, account_name: string|null, account_code: int|null, amount: string, debit: string|null, credit: string|null }] }  // Filters combine. account_id and account include descendant accounts. source_ref_id requires source_doc_type. side debit keeps positive and zero amounts. from and to are inclusive ISO datetimes or calendar dates, interpreted in UTC.",
        |_ctx| NativeBinding::Function(Arc::new(list_journal_entry_items)),
    );
    rune_env.register_contextual(
        "get_source_doc",
        "get_source_doc(#{ id: int }) -> #{ id: int, source_doc_type: string, source_ref_id: int|null, journal_entries: [#{ id: int, datetime: string, journal_id: int, journal_name: string|null, source_doc_id: int, source_doc_type: string|null, source_ref_id: int|null, transferred_amount: string, lines: [#{ id: int, account_id: int, account_name: string|null, account_code: int|null, amount: string, debit: string|null, credit: string|null }] }] }  // id is the source_docs row. source_ref_id is the business document id.",
        |_ctx| NativeBinding::Function(Arc::new(get_source_doc)),
    );
    rune_env.register_contextual(
        "list_source_docs",
        "list_source_docs(#{ source_doc_type?: string, source_ref_id?: int, source_doc_id?: int, query?: string, limit?: int }) -> #{ results: [#{ id: int, source_doc_type: string, source_ref_id: int|null, journal_entries: [#{ id: int, datetime: string, journal_id: int, journal_name: string|null, source_doc_id: int, source_doc_type: string|null, source_ref_id: int|null, transferred_amount: string, lines: [#{ id: int, account_id: int, account_name: string|null, account_code: int|null, amount: string, debit: string|null, credit: string|null }] }] }] }  // Provide at least one filter. source_doc_id is the source_docs row. source_ref_id is the business document id and requires source_doc_type. query matches the type, a custom transaction title, a source_docs row id, or a business document id.",
        |_ctx| NativeBinding::Function(Arc::new(list_source_docs)),
    );
}

#[cfg(feature = "cap-llm")]
fn get_account(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = args::require_id(
        args::parse_id(args, "get_account")?,
        "get_account",
        "account",
    )?;
    let db = ctx.db.clone();
    let account = lariv_core::rune_env::block_on_async(async move {
        let account = rows::find_account(&db, id).await?;
        let balance = rows::account_balance(&db, account.id).await?;
        let ancestors = rows::account_ancestors(&db, account.parent_id).await?;
        let children = rows::account_children(&db, account.id).await?;
        Ok::<_, String>(rows::account_detail(
            &account, &balance, ancestors, children,
        ))
    })?;
    rows::to_rune(account)
}

#[cfg(feature = "cap-llm")]
fn search_accounts(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let parsed = args::parse_account_search(args)?;
    let db = ctx.db.clone();
    let accounts =
        lariv_core::rune_env::block_on_async(
            async move { rows::search_accounts(&db, &parsed).await },
        )?;
    rows::to_rune(serde_json::json!({ "results": accounts }))
}

#[cfg(feature = "cap-llm")]
fn list_accounts(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let parsed = args::parse_account_list(args)?;
    let db = ctx.db.clone();
    let accounts =
        lariv_core::rune_env::block_on_async(
            async move { rows::list_accounts(&db, &parsed).await },
        )?;
    rows::to_rune(serde_json::json!({ "results": accounts }))
}

#[cfg(feature = "cap-llm")]
fn get_journal_entry(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = args::require_id(
        args::parse_id(args, "get_journal_entry")?,
        "get_journal_entry",
        "journal entry",
    )?;
    let db = ctx.db.clone();
    let entry = lariv_core::rune_env::block_on_async(async move {
        let entry = rows::find_entry(&db, id).await?;
        let mut presented = rows::present_entries(&db, vec![entry]).await?;
        presented
            .pop()
            .ok_or_else(|| "journal entry not found".to_string())
    })?;
    rows::to_rune(entry)
}

#[cfg(feature = "cap-llm")]
fn list_journal_entries(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let parsed = args::parse_entry_list(args)?;
    let db = ctx.db.clone();
    let entries = lariv_core::rune_env::block_on_async(async move {
        rows::list_journal_entries(&db, &parsed).await
    })?;
    rows::to_rune(serde_json::json!({ "results": entries }))
}

#[cfg(feature = "cap-llm")]
fn get_journal_entry_item(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = args::require_id(
        args::parse_id(args, "get_journal_entry_item")?,
        "get_journal_entry_item",
        "journal entry item",
    )?;
    let db = ctx.db.clone();
    let item = lariv_core::rune_env::block_on_async(async move {
        let item = rows::find_item(&db, id).await?;
        let mut presented = rows::present_items(&db, vec![item]).await?;
        presented
            .pop()
            .ok_or_else(|| "journal entry item not found".to_string())
    })?;
    rows::to_rune(item)
}

#[cfg(feature = "cap-llm")]
fn list_journal_entry_items(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let parsed = args::parse_item_list(args)?;
    let db = ctx.db.clone();
    let items = lariv_core::rune_env::block_on_async(async move {
        rows::list_journal_entry_items(&db, &parsed).await
    })?;
    rows::to_rune(serde_json::json!({ "results": items }))
}

#[cfg(feature = "cap-llm")]
fn get_source_doc(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let id = args::require_id(
        args::parse_id(args, "get_source_doc")?,
        "get_source_doc",
        "source doc",
    )?;
    let db = ctx.db.clone();
    let doc = lariv_core::rune_env::block_on_async(async move {
        let doc = rows::find_source_doc(&db, id).await?;
        let docs = rows::present_source_docs(&db, vec![doc]).await?;
        docs.into_iter()
            .next()
            .ok_or_else(|| "source doc not found".to_string())
    })?;
    rows::to_rune(doc)
}

#[cfg(feature = "cap-llm")]
fn list_source_docs(
    ctx: &lariv_core::rune_env::RuneEnvCtx<'_>,
    args: &[rune::Value],
) -> Result<rune::Value, String> {
    let parsed = args::parse_source_list(args)?;
    let db = ctx.db.clone();
    let docs =
        lariv_core::rune_env::block_on_async(
            async move { rows::list_source_docs(&db, &parsed).await },
        )?;
    rows::to_rune(serde_json::json!({ "results": docs }))
}

#[cfg(feature = "cap-llm")]
mod args {
    use chrono::{DateTime, NaiveTime, Utc};
    use serde::Deserialize;

    use crate::balance_type::BalanceType;

    use super::rows::Limit;

    #[derive(Debug, Deserialize)]
    struct IdArgs {
        id: i64,
    }

    pub(super) fn object_arg<'a>(
        args: &'a [rune::Value],
        op: &str,
    ) -> Result<&'a rune::Value, String> {
        args.first()
            .ok_or_else(|| format!("{op} requires an object argument"))
    }

    pub(super) fn parse_json<T: serde::de::DeserializeOwned>(
        value: &rune::Value,
        op: &str,
    ) -> Result<T, String> {
        serde_json::from_value(
            lariv_core::rune_env::rune_to_json(value)
                .map_err(|e| format!("invalid {op} arguments: {e}"))?,
        )
        .map_err(|e| format!("invalid {op} arguments: {e}"))
    }

    pub(super) fn parse_id(args: &[rune::Value], op: &str) -> Result<i64, String> {
        let parsed: IdArgs = parse_json(object_arg(args, op)?, op)?;
        Ok(parsed.id)
    }

    pub(super) fn require_id(id: i64, op: &str, label: &str) -> Result<i64, String> {
        if id <= 0 {
            Err(format!("{op} requires a positive {label} id"))
        } else {
            Ok(id)
        }
    }

    pub(super) fn parse_balance_type(raw: &str, op: &str) -> Result<BalanceType, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "debit" => Ok(BalanceType::Debit),
            "credit" => Ok(BalanceType::Credit),
            _ => Err(format!("{op} balance_type must be Debit or Credit")),
        }
    }

    pub(super) fn parse_limit(limit: Option<i64>, op: &str) -> Result<Limit, String> {
        match limit {
            None => Ok(Limit(lariv_core::db::trigram::clamp_search_limit(0))),
            Some(value) if value <= 0 => Err(format!("{op} limit must be positive")),
            Some(value) => Ok(Limit(lariv_core::db::trigram::clamp_search_limit(
                u64::try_from(value).unwrap_or(u64::MAX),
            ))),
        }
    }

    /// Inclusive UTC bound. A calendar date covers that whole UTC day when `end_of_day`.
    pub(super) fn parse_bound(raw: &str, end_of_day: bool) -> Result<DateTime<Utc>, String> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err("requires a datetime".to_string());
        }
        if let Ok(parsed) = DateTime::parse_from_rfc3339(raw) {
            return Ok(parsed.with_timezone(&Utc));
        }
        if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S") {
            return Ok(parsed.and_utc());
        }
        if let Some(date) = lariv_core::datetime::parse_date(raw) {
            let time = if end_of_day {
                NaiveTime::from_hms_opt(23, 59, 59).unwrap_or(NaiveTime::MIN)
            } else {
                NaiveTime::MIN
            };
            return Ok(date.and_time(time).and_utc());
        }
        Err(format!("invalid datetime: {raw}"))
    }

    fn optional_text(value: Option<String>) -> Option<String> {
        value.and_then(|text| {
            let trimmed = text.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    }

    #[derive(Debug, Deserialize)]
    struct AccountSearchArgs {
        #[serde(default)]
        query: String,
        #[serde(default)]
        limit: Option<i64>,
        #[serde(default)]
        balance_type: Option<String>,
        #[serde(default)]
        parent_id: Option<i64>,
        #[serde(default)]
        leaves_only: bool,
    }

    #[derive(Debug)]
    pub(super) struct AccountSearch {
        pub query: String,
        pub limit: Limit,
        pub balance_type: Option<BalanceType>,
        pub parent_id: Option<i64>,
        pub leaves_only: bool,
    }

    pub(super) fn parse_account_search(args: &[rune::Value]) -> Result<AccountSearch, String> {
        let parsed: AccountSearchArgs =
            parse_json(object_arg(args, "search_accounts")?, "search_accounts")?;
        let query = parsed.query.trim().to_string();
        if query.is_empty() {
            return Err("search_accounts requires query".to_string());
        }
        let parent_id = match parsed.parent_id {
            Some(id) if id <= 0 => {
                return Err("search_accounts parent_id must be positive".to_string());
            }
            other => other,
        };
        Ok(AccountSearch {
            query,
            limit: parse_limit(parsed.limit, "search_accounts")?,
            balance_type: parsed
                .balance_type
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| parse_balance_type(value, "search_accounts"))
                .transpose()?,
            parent_id,
            leaves_only: parsed.leaves_only,
        })
    }

    #[derive(Debug, Deserialize)]
    struct AccountListArgs {
        #[serde(default)]
        parent_id: Option<i64>,
        #[serde(default)]
        balance_type: Option<String>,
        #[serde(default)]
        leaves_only: bool,
        #[serde(default)]
        limit: Option<i64>,
    }

    #[derive(Debug)]
    pub(super) struct AccountList {
        pub parent_id: Option<i64>,
        pub balance_type: Option<BalanceType>,
        pub leaves_only: bool,
        pub limit: Limit,
    }

    pub(super) fn parse_account_list(args: &[rune::Value]) -> Result<AccountList, String> {
        let parsed: AccountListArgs =
            parse_json(object_arg(args, "list_accounts")?, "list_accounts")?;
        if let Some(id) = parsed.parent_id
            && id < 0
        {
            return Err("list_accounts parent_id must be zero or positive".to_string());
        }
        Ok(AccountList {
            parent_id: parsed.parent_id,
            balance_type: parsed
                .balance_type
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| parse_balance_type(value, "list_accounts"))
                .transpose()?,
            leaves_only: parsed.leaves_only,
            limit: parse_limit(parsed.limit, "list_accounts")?,
        })
    }

    #[derive(Debug, Deserialize, Default)]
    struct EntryListArgs {
        #[serde(default)]
        journal_id: Option<i64>,
        #[serde(default)]
        journal: Option<String>,
        #[serde(default)]
        account_id: Option<i64>,
        #[serde(default)]
        account: Option<String>,
        #[serde(default)]
        source_doc_id: Option<i64>,
        #[serde(default)]
        source_doc_type: Option<String>,
        #[serde(default)]
        source_ref_id: Option<i64>,
        #[serde(default)]
        from: Option<String>,
        #[serde(default)]
        to: Option<String>,
        #[serde(default)]
        limit: Option<i64>,
    }

    #[derive(Debug)]
    pub(super) struct EntryList {
        pub journal_id: Option<i64>,
        pub journal: Option<String>,
        pub account_id: Option<i64>,
        pub account: Option<String>,
        pub source_doc_id: Option<i64>,
        pub source_doc_type: Option<String>,
        pub source_ref_id: Option<i64>,
        pub from: Option<DateTime<Utc>>,
        pub to: Option<DateTime<Utc>>,
        pub limit: Limit,
    }

    fn parse_entry_fields(parsed: EntryListArgs, op: &str) -> Result<EntryList, String> {
        if let Some(id) = parsed.journal_id {
            require_id(id, op, "journal")?;
        }
        if let Some(id) = parsed.account_id {
            require_id(id, op, "account")?;
        }
        if let Some(id) = parsed.source_doc_id {
            require_id(id, op, "source doc")?;
        }
        if let Some(id) = parsed.source_ref_id {
            require_id(id, op, "source document")?;
        }
        let source_doc_type = optional_text(parsed.source_doc_type);
        if parsed.source_ref_id.is_some() && source_doc_type.is_none() {
            return Err(format!("{op} source_ref_id requires source_doc_type"));
        }
        let from = parsed
            .from
            .as_deref()
            .map(|value| parse_bound(value, false))
            .transpose()
            .map_err(|error| format!("{op} {error}"))?;
        let to = parsed
            .to
            .as_deref()
            .map(|value| parse_bound(value, true))
            .transpose()
            .map_err(|error| format!("{op} {error}"))?;
        if let (Some(from), Some(to)) = (from, to)
            && from > to
        {
            return Err(format!("{op} from is after to"));
        }
        Ok(EntryList {
            journal_id: parsed.journal_id,
            journal: optional_text(parsed.journal),
            account_id: parsed.account_id,
            account: optional_text(parsed.account),
            source_doc_id: parsed.source_doc_id,
            source_doc_type,
            source_ref_id: parsed.source_ref_id,
            from,
            to,
            limit: parse_limit(parsed.limit, op)?,
        })
    }

    pub(super) fn parse_entry_list(args: &[rune::Value]) -> Result<EntryList, String> {
        let parsed: EntryListArgs = parse_json(
            object_arg(args, "list_journal_entries")?,
            "list_journal_entries",
        )?;
        parse_entry_fields(parsed, "list_journal_entries")
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) enum Side {
        Debit,
        Credit,
    }

    fn parse_side(raw: &str, op: &str) -> Result<Side, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "debit" => Ok(Side::Debit),
            "credit" => Ok(Side::Credit),
            _ => Err(format!("{op} side must be debit or credit")),
        }
    }

    #[derive(Debug, Deserialize, Default)]
    struct ItemListArgs {
        #[serde(flatten)]
        entry: EntryListArgs,
        #[serde(default)]
        journal_entry_id: Option<i64>,
        #[serde(default)]
        side: Option<String>,
    }

    #[derive(Debug)]
    pub(super) struct ItemList {
        pub entry: EntryList,
        pub journal_entry_id: Option<i64>,
        pub side: Option<Side>,
    }

    pub(super) fn parse_item_list(args: &[rune::Value]) -> Result<ItemList, String> {
        let parsed: ItemListArgs = parse_json(
            object_arg(args, "list_journal_entry_items")?,
            "list_journal_entry_items",
        )?;
        if let Some(id) = parsed.journal_entry_id {
            require_id(id, "list_journal_entry_items", "journal entry")?;
        }
        let side = parsed
            .side
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| parse_side(value, "list_journal_entry_items"))
            .transpose()?;
        Ok(ItemList {
            entry: parse_entry_fields(parsed.entry, "list_journal_entry_items")?,
            journal_entry_id: parsed.journal_entry_id,
            side,
        })
    }

    #[derive(Debug, Deserialize, Default)]
    struct SourceListArgs {
        #[serde(default)]
        source_doc_type: Option<String>,
        #[serde(default)]
        source_ref_id: Option<i64>,
        #[serde(default)]
        source_doc_id: Option<i64>,
        #[serde(default)]
        query: Option<String>,
        #[serde(default)]
        limit: Option<i64>,
    }

    #[derive(Debug)]
    pub(super) struct SourceList {
        pub source_doc_type: Option<String>,
        pub source_ref_id: Option<i64>,
        pub source_doc_id: Option<i64>,
        pub query: Option<String>,
        pub limit: Limit,
    }

    pub(super) fn parse_source_list(args: &[rune::Value]) -> Result<SourceList, String> {
        let parsed: SourceListArgs =
            parse_json(object_arg(args, "list_source_docs")?, "list_source_docs")?;
        if let Some(id) = parsed.source_doc_id {
            require_id(id, "list_source_docs", "source doc")?;
        }
        if let Some(id) = parsed.source_ref_id {
            require_id(id, "list_source_docs", "source document")?;
        }
        let source_doc_type = optional_text(parsed.source_doc_type);
        let query = optional_text(parsed.query);
        if parsed.source_ref_id.is_some() && source_doc_type.is_none() {
            return Err("list_source_docs source_ref_id requires source_doc_type".to_string());
        }
        if parsed.source_doc_id.is_none()
            && parsed.source_ref_id.is_none()
            && source_doc_type.is_none()
            && query.is_none()
        {
            return Err(
                "list_source_docs requires source_doc_type, source_ref_id, source_doc_id, or query"
                    .to_string(),
            );
        }
        Ok(SourceList {
            source_doc_type,
            source_ref_id: parsed.source_ref_id,
            source_doc_id: parsed.source_doc_id,
            query,
            limit: parse_limit(parsed.limit, "list_source_docs")?,
        })
    }
}

#[cfg(feature = "cap-llm")]
mod rows {
    use std::collections::{HashMap, HashSet};

    use rust_decimal::Decimal;
    use sea_orm::{
        ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
        QuerySelect,
    };

    use lariv_formula::decimal::{self, normalize};

    use crate::account_validation::account_descendant_ids;
    use crate::balance_type::BalanceType;
    use crate::entities::custom_source_doc::{self, CUSTOM_SOURCE_DOC_TYPE};
    use crate::entities::{
        account::{self, Entity as AccountEntity},
        journal::{self, Entity as JournalEntity},
        journal_entry::{self, Entity as JournalEntryEntity},
        journal_entry_item::{self, Entity as JournalEntryItemEntity},
        source_doc::{self, Entity as SourceDocEntity},
    };

    use super::args::{AccountList, AccountSearch, EntryList, ItemList, Side, SourceList};

    #[derive(Debug)]
    pub(super) struct Limit(pub u64);

    pub(super) fn to_rune(value: serde_json::Value) -> Result<rune::Value, String> {
        lariv_core::rune_env::json_to_rune(value)
    }

    pub(super) fn money(amount: Decimal) -> String {
        normalize(amount).to_string()
    }

    /// Debit string and credit string. A zero amount is a debit of zero.
    pub(super) fn split_amount(amount: Decimal) -> (Option<String>, Option<String>) {
        if amount.is_sign_negative() {
            (None, Some(money(decimal::dec_abs(amount))))
        } else {
            (Some(money(amount)), None)
        }
    }

    fn account_row(account: &account::Model, balance: &str) -> serde_json::Value {
        serde_json::json!({
            "id": account.id,
            "name": account.name,
            "code": account.code,
            "is_group": account.is_group,
            "balance_type": account.balance_type.as_str(),
            "parent_id": account.parent_id,
            "balance": balance,
        })
    }

    pub(super) fn account_detail(
        account: &account::Model,
        balance: &str,
        ancestors: Vec<serde_json::Value>,
        children: Vec<serde_json::Value>,
    ) -> serde_json::Value {
        let mut row = account_row(account, balance);
        if let Some(object) = row.as_object_mut() {
            object.insert("ancestors".to_string(), serde_json::Value::Array(ancestors));
            object.insert("children".to_string(), serde_json::Value::Array(children));
        }
        row
    }

    pub(super) async fn find_account(
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<account::Model, String> {
        AccountEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "account not found".to_string())
    }

    pub(super) async fn find_entry(
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<journal_entry::Model, String> {
        JournalEntryEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "journal entry not found".to_string())
    }

    pub(super) async fn find_source_doc(
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<source_doc::Model, String> {
        SourceDocEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "source doc not found".to_string())
    }

    pub(super) async fn account_balance(
        db: &DatabaseConnection,
        account_id: i64,
    ) -> Result<String, String> {
        let ids = account_descendant_ids(db, account_id)
            .await
            .map_err(|error| error.to_string())?;
        if ids.is_empty() {
            return Ok(money(Decimal::ZERO));
        }
        let sum: Option<Decimal> = JournalEntryItemEntity::find()
            .filter(journal_entry_item::Column::AccountId.is_in(ids))
            .select_only()
            .column_as(
                sea_orm::sea_query::Expr::cust("COALESCE(SUM(amount), 0)"),
                "total",
            )
            .into_tuple()
            .one(db)
            .await
            .map_err(|error| error.to_string())?;
        Ok(money(sum.unwrap_or(Decimal::ZERO)))
    }

    pub(super) async fn account_ancestors(
        db: &DatabaseConnection,
        mut parent_id: Option<i64>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut chain = Vec::new();
        let mut seen = HashSet::new();
        for _ in 0..64 {
            let Some(parent) = parent_id.filter(|id| *id > 0) else {
                break;
            };
            if !seen.insert(parent) {
                break;
            }
            let Some(account) = AccountEntity::find_by_id(parent)
                .one(db)
                .await
                .map_err(|error| error.to_string())?
            else {
                break;
            };
            parent_id = account.parent_id;
            chain.push(serde_json::json!({
                "id": account.id,
                "name": account.name,
                "code": account.code,
            }));
        }
        chain.reverse();
        Ok(chain)
    }

    pub(super) async fn account_children(
        db: &DatabaseConnection,
        account_id: i64,
    ) -> Result<Vec<serde_json::Value>, String> {
        let children = AccountEntity::find()
            .filter(account::Column::ParentId.eq(account_id))
            .order_by_asc(account::Column::Code)
            .order_by_asc(account::Column::Id)
            .all(db)
            .await
            .map_err(|error| error.to_string())?;
        Ok(children
            .iter()
            .map(|account| {
                serde_json::json!({
                    "id": account.id,
                    "name": account.name,
                    "code": account.code,
                    "is_group": account.is_group,
                    "balance_type": account.balance_type.as_str(),
                })
            })
            .collect())
    }

    async fn with_balances(
        db: &DatabaseConnection,
        accounts: Vec<account::Model>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut rows = Vec::with_capacity(accounts.len());
        for account in accounts {
            let balance = account_balance(db, account.id).await?;
            rows.push(account_row(&account, &balance));
        }
        Ok(rows)
    }

    fn account_query(
        balance_type: Option<BalanceType>,
        leaves_only: bool,
    ) -> sea_orm::Select<account::Entity> {
        let mut query = AccountEntity::find();
        if let Some(balance_type) = balance_type {
            query = query.filter(account::Column::BalanceType.eq(balance_type));
        }
        if leaves_only {
            query = query.filter(account::Column::IsGroup.eq(false));
        }
        query
    }

    pub(super) async fn search_accounts(
        db: &DatabaseConnection,
        parsed: &AccountSearch,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut query = account_query(parsed.balance_type, parsed.leaves_only);
        if let Some(parent_id) = parsed.parent_id {
            find_account(db, parent_id).await?;
            let mut ids = account_descendant_ids(db, parent_id)
                .await
                .map_err(|error| error.to_string())?;
            ids.retain(|id| *id != parent_id);
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            query = query.filter(account::Column::Id.is_in(ids));
        }
        let name_match = lariv_core::db::trigram::text_match_condition(
            db.get_database_backend(),
            &[account::Column::Name],
            &parsed.query,
        );
        let matched = match parsed.query.parse::<i32>() {
            Ok(code) => Condition::any()
                .add(account::Column::Code.eq(code))
                .add(name_match),
            Err(_) => name_match,
        };
        let accounts = query
            .filter(matched)
            .order_by_asc(account::Column::Code)
            .order_by_asc(account::Column::Id)
            .limit(parsed.limit.0)
            .all(db)
            .await
            .map_err(|error| error.to_string())?;
        with_balances(db, accounts).await
    }

    pub(super) async fn list_accounts(
        db: &DatabaseConnection,
        parsed: &AccountList,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut query = account_query(parsed.balance_type, parsed.leaves_only);
        match parsed.parent_id {
            Some(0) => {
                query = query.filter(account::Column::ParentId.is_null());
            }
            Some(parent_id) => {
                find_account(db, parent_id).await?;
                query = query.filter(account::Column::ParentId.eq(parent_id));
            }
            None => {}
        }
        let accounts = query
            .order_by_asc(account::Column::Code)
            .order_by_asc(account::Column::Id)
            .limit(parsed.limit.0)
            .all(db)
            .await
            .map_err(|error| error.to_string())?;
        with_balances(db, accounts).await
    }

    fn line_json(
        item: &journal_entry_item::Model,
        accounts: &HashMap<i64, account::Model>,
    ) -> serde_json::Value {
        let account = accounts.get(&item.account_id);
        let (debit, credit) = split_amount(item.amount);
        serde_json::json!({
            "id": item.id,
            "account_id": item.account_id,
            "account_name": account.map(|row| row.name.clone()),
            "account_code": account.map(|row| row.code),
            "amount": money(item.amount),
            "debit": debit,
            "credit": credit,
        })
    }

    fn ref_id(id: i64) -> Option<i64> {
        if id > 0 { Some(id) } else { None }
    }

    fn entry_json(
        entry: &journal_entry::Model,
        journal_name: Option<&str>,
        source_doc_type: Option<&str>,
        source_ref_id: Option<i64>,
        lines: Vec<serde_json::Value>,
        transferred_amount: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "id": entry.id,
            "datetime": entry.datetime.to_rfc3339(),
            "journal_id": entry.journal_id,
            "journal_name": journal_name,
            "source_doc_id": entry.source_doc_id,
            "source_doc_type": source_doc_type,
            "source_ref_id": source_ref_id,
            "transferred_amount": transferred_amount,
            "lines": lines,
        })
    }

    /// Sum of the debit lines. On a balanced entry this equals the credit total.
    pub(super) fn transferred_amount(amounts: impl IntoIterator<Item = Decimal>) -> String {
        let mut total = Decimal::ZERO;
        for amount in amounts {
            if !amount.is_sign_negative() {
                total = decimal::dec_sum(total, amount);
            }
        }
        money(total)
    }

    fn dedup_ids(ids: impl IntoIterator<Item = i64>) -> Vec<i64> {
        let mut ids: Vec<i64> = ids.into_iter().filter(|id| *id > 0).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    pub(super) async fn present_entries(
        db: &DatabaseConnection,
        entries: Vec<journal_entry::Model>,
    ) -> Result<Vec<serde_json::Value>, String> {
        if entries.is_empty() {
            return Ok(Vec::new());
        }
        let entry_ids = dedup_ids(entries.iter().map(|entry| entry.id));
        let items = JournalEntryItemEntity::find()
            .filter(journal_entry_item::Column::JournalEntryId.is_in(entry_ids))
            .order_by_asc(journal_entry_item::Column::Id)
            .all(db)
            .await
            .map_err(|error| error.to_string())?;
        let account_ids = dedup_ids(items.iter().map(|item| item.account_id));
        let accounts = if account_ids.is_empty() {
            HashMap::new()
        } else {
            AccountEntity::find()
                .filter(account::Column::Id.is_in(account_ids))
                .all(db)
                .await
                .map_err(|error| error.to_string())?
                .into_iter()
                .map(|account| (account.id, account))
                .collect::<HashMap<_, _>>()
        };
        let journal_ids = dedup_ids(entries.iter().map(|entry| entry.journal_id));
        let journals = if journal_ids.is_empty() {
            HashMap::new()
        } else {
            JournalEntity::find()
                .filter(journal::Column::Id.is_in(journal_ids))
                .all(db)
                .await
                .map_err(|error| error.to_string())?
                .into_iter()
                .map(|journal| (journal.id, journal.name))
                .collect::<HashMap<_, _>>()
        };
        let doc_ids = dedup_ids(entries.iter().map(|entry| entry.source_doc_id));
        let docs = if doc_ids.is_empty() {
            HashMap::new()
        } else {
            SourceDocEntity::find()
                .filter(source_doc::Column::Id.is_in(doc_ids))
                .all(db)
                .await
                .map_err(|error| error.to_string())?
                .into_iter()
                .map(|doc| (doc.id, doc))
                .collect::<HashMap<_, _>>()
        };
        let mut items_by_entry: HashMap<i64, Vec<journal_entry_item::Model>> = HashMap::new();
        for item in items {
            items_by_entry
                .entry(item.journal_entry_id)
                .or_default()
                .push(item);
        }
        let mut presented = Vec::with_capacity(entries.len());
        for entry in &entries {
            let entry_items = items_by_entry.remove(&entry.id).unwrap_or_default();
            let lines = entry_items
                .iter()
                .map(|item| line_json(item, &accounts))
                .collect();
            let transferred = transferred_amount(entry_items.iter().map(|item| item.amount));
            let doc = docs.get(&entry.source_doc_id);
            presented.push(entry_json(
                entry,
                journals.get(&entry.journal_id).map(String::as_str),
                doc.map(|row| row.source_doc_type.as_str()),
                doc.and_then(|row| ref_id(row.source_doc_id)),
                lines,
                &transferred,
            ));
        }
        Ok(presented)
    }

    async fn entry_ids_for_accounts(
        db: &DatabaseConnection,
        account_ids: &[i64],
    ) -> Result<HashSet<i64>, String> {
        let mut accounts = Vec::new();
        for account_id in account_ids {
            let descendants = account_descendant_ids(db, *account_id)
                .await
                .map_err(|error| error.to_string())?;
            accounts.extend(descendants);
        }
        let accounts = dedup_ids(accounts);
        if accounts.is_empty() {
            return Ok(HashSet::new());
        }
        let items = JournalEntryItemEntity::find()
            .filter(journal_entry_item::Column::AccountId.is_in(accounts))
            .all(db)
            .await
            .map_err(|error| error.to_string())?;
        Ok(items
            .into_iter()
            .map(|item| item.journal_entry_id)
            .collect())
    }

    fn intersect(current: Option<HashSet<i64>>, more: HashSet<i64>) -> HashSet<i64> {
        match current {
            Some(existing) => existing.intersection(&more).copied().collect(),
            None => more,
        }
    }

    async fn accounts_matching(db: &DatabaseConnection, query: &str) -> Result<Vec<i64>, String> {
        let mut ids = Vec::new();
        let named = lariv_core::db::trigram::search::<account::Entity, _>(
            db,
            &[account::Column::Name],
            query,
            lariv_core::db::trigram::MAX_SEARCH_LIMIT,
        )
        .await
        .map_err(|error| error.to_string())?;
        ids.extend(named.into_iter().map(|account| account.id));
        if let Ok(code) = query.parse::<i32>() {
            let coded = AccountEntity::find()
                .filter(account::Column::Code.eq(code))
                .all(db)
                .await
                .map_err(|error| error.to_string())?;
            ids.extend(coded.into_iter().map(|account| account.id));
        }
        Ok(dedup_ids(ids))
    }

    async fn source_doc_entry_ids(
        db: &DatabaseConnection,
        parsed: &EntryList,
    ) -> Result<Option<HashSet<i64>>, String> {
        if parsed.source_doc_id.is_none()
            && parsed.source_doc_type.is_none()
            && parsed.source_ref_id.is_none()
        {
            return Ok(None);
        }
        let mut docs = SourceDocEntity::find();
        if let Some(id) = parsed.source_doc_id {
            find_source_doc(db, id).await?;
            docs = docs.filter(source_doc::Column::Id.eq(id));
        }
        if let Some(source_doc_type) = parsed.source_doc_type.as_deref() {
            docs = docs.filter(source_doc::Column::SourceDocType.eq(source_doc_type));
        }
        if let Some(source_ref_id) = parsed.source_ref_id {
            docs = docs.filter(source_doc::Column::SourceDocId.eq(source_ref_id));
        }
        let docs = docs.all(db).await.map_err(|error| error.to_string())?;
        if docs.is_empty() {
            return Ok(Some(HashSet::new()));
        }
        let doc_ids = dedup_ids(docs.iter().map(|doc| doc.id));
        let entries = JournalEntryEntity::find()
            .filter(journal_entry::Column::SourceDocId.is_in(doc_ids))
            .all(db)
            .await
            .map_err(|error| error.to_string())?;
        Ok(Some(entries.into_iter().map(|entry| entry.id).collect()))
    }

    fn entry_scope(parsed: &EntryList) -> bool {
        parsed.journal_id.is_some()
            || parsed.journal.is_some()
            || parsed.account_id.is_some()
            || parsed.account.is_some()
            || parsed.source_doc_id.is_some()
            || parsed.source_doc_type.is_some()
            || parsed.source_ref_id.is_some()
            || parsed.from.is_some()
            || parsed.to.is_some()
    }

    async fn select_entries(
        db: &DatabaseConnection,
        parsed: &EntryList,
    ) -> Result<Option<sea_orm::Select<journal_entry::Entity>>, String> {
        let mut allowed: Option<HashSet<i64>> = None;
        if let Some(account_id) = parsed.account_id {
            find_account(db, account_id).await?;
            allowed = Some(entry_ids_for_accounts(db, &[account_id]).await?);
        }
        if let Some(account) = parsed.account.as_deref() {
            let matched = accounts_matching(db, account).await?;
            if matched.is_empty() {
                return Ok(None);
            }
            allowed = Some(intersect(
                allowed,
                entry_ids_for_accounts(db, &matched).await?,
            ));
        }
        if let Some(doc_entries) = source_doc_entry_ids(db, parsed).await? {
            allowed = Some(intersect(allowed, doc_entries));
        }
        if let Some(ids) = allowed.as_ref()
            && ids.is_empty()
        {
            return Ok(None);
        }

        let mut query = JournalEntryEntity::find();
        if let Some(journal_id) = parsed.journal_id {
            JournalEntity::find_by_id(journal_id)
                .one(db)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "journal not found".to_string())?;
            query = query.filter(journal_entry::Column::JournalId.eq(journal_id));
        }
        if let Some(journal_name) = parsed.journal.as_deref() {
            let journals = lariv_core::db::trigram::search::<journal::Entity, _>(
                db,
                &[journal::Column::Name],
                journal_name,
                lariv_core::db::trigram::MAX_SEARCH_LIMIT,
            )
            .await
            .map_err(|error| error.to_string())?;
            if journals.is_empty() {
                return Ok(None);
            }
            let journal_ids = dedup_ids(journals.into_iter().map(|journal| journal.id));
            query = query.filter(journal_entry::Column::JournalId.is_in(journal_ids));
        }
        if let Some(ids) = allowed {
            let ids = dedup_ids(ids);
            if ids.is_empty() {
                return Ok(None);
            }
            query = query.filter(journal_entry::Column::Id.is_in(ids));
        }
        if let Some(from) = parsed.from {
            query = query.filter(journal_entry::Column::Datetime.gte(from));
        }
        if let Some(to) = parsed.to {
            query = query.filter(journal_entry::Column::Datetime.lte(to));
        }
        Ok(Some(query))
    }

    pub(super) async fn list_journal_entries(
        db: &DatabaseConnection,
        parsed: &EntryList,
    ) -> Result<Vec<serde_json::Value>, String> {
        let Some(query) = select_entries(db, parsed).await? else {
            return Ok(Vec::new());
        };
        let entries = query
            .order_by_desc(journal_entry::Column::Datetime)
            .order_by_desc(journal_entry::Column::Id)
            .limit(parsed.limit.0)
            .all(db)
            .await
            .map_err(|error| error.to_string())?;
        present_entries(db, entries).await
    }

    pub(super) async fn find_item(
        db: &DatabaseConnection,
        id: i64,
    ) -> Result<journal_entry_item::Model, String> {
        JournalEntryItemEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "journal entry item not found".to_string())
    }

    fn item_json(
        item: &journal_entry_item::Model,
        accounts: &HashMap<i64, account::Model>,
        entry: Option<&journal_entry::Model>,
        journal_name: Option<&str>,
        source_doc_type: Option<&str>,
        source_ref_id: Option<i64>,
    ) -> serde_json::Value {
        let account = accounts.get(&item.account_id);
        let (debit, credit) = split_amount(item.amount);
        serde_json::json!({
            "id": item.id,
            "datetime": item.datetime.to_rfc3339(),
            "journal_entry_id": item.journal_entry_id,
            "journal_id": entry.map(|row| row.journal_id),
            "journal_name": journal_name,
            "source_doc_id": entry.map(|row| row.source_doc_id),
            "source_doc_type": source_doc_type,
            "source_ref_id": source_ref_id,
            "account_id": item.account_id,
            "account_name": account.map(|row| row.name.clone()),
            "account_code": account.map(|row| row.code),
            "amount": money(item.amount),
            "debit": debit,
            "credit": credit,
        })
    }

    pub(super) async fn present_items(
        db: &DatabaseConnection,
        items: Vec<journal_entry_item::Model>,
    ) -> Result<Vec<serde_json::Value>, String> {
        if items.is_empty() {
            return Ok(Vec::new());
        }
        let account_ids = dedup_ids(items.iter().map(|item| item.account_id));
        let accounts = if account_ids.is_empty() {
            HashMap::new()
        } else {
            AccountEntity::find()
                .filter(account::Column::Id.is_in(account_ids))
                .all(db)
                .await
                .map_err(|error| error.to_string())?
                .into_iter()
                .map(|account| (account.id, account))
                .collect::<HashMap<_, _>>()
        };
        let entry_ids = dedup_ids(items.iter().map(|item| item.journal_entry_id));
        let entries = if entry_ids.is_empty() {
            HashMap::new()
        } else {
            JournalEntryEntity::find()
                .filter(journal_entry::Column::Id.is_in(entry_ids))
                .all(db)
                .await
                .map_err(|error| error.to_string())?
                .into_iter()
                .map(|entry| (entry.id, entry))
                .collect::<HashMap<_, _>>()
        };
        let journal_ids = dedup_ids(entries.values().map(|entry| entry.journal_id));
        let journals = if journal_ids.is_empty() {
            HashMap::new()
        } else {
            JournalEntity::find()
                .filter(journal::Column::Id.is_in(journal_ids))
                .all(db)
                .await
                .map_err(|error| error.to_string())?
                .into_iter()
                .map(|journal| (journal.id, journal.name))
                .collect::<HashMap<_, _>>()
        };
        let doc_ids = dedup_ids(entries.values().map(|entry| entry.source_doc_id));
        let docs = if doc_ids.is_empty() {
            HashMap::new()
        } else {
            SourceDocEntity::find()
                .filter(source_doc::Column::Id.is_in(doc_ids))
                .all(db)
                .await
                .map_err(|error| error.to_string())?
                .into_iter()
                .map(|doc| (doc.id, doc))
                .collect::<HashMap<_, _>>()
        };
        Ok(items
            .iter()
            .map(|item| {
                let entry = entries.get(&item.journal_entry_id);
                let doc = entry.and_then(|row| docs.get(&row.source_doc_id));
                let journal_name = entry.and_then(|row| journals.get(&row.journal_id));
                item_json(
                    item,
                    &accounts,
                    entry,
                    journal_name.map(String::as_str),
                    doc.map(|row| row.source_doc_type.as_str()),
                    doc.and_then(|row| ref_id(row.source_doc_id)),
                )
            })
            .collect())
    }

    pub(super) async fn list_journal_entry_items(
        db: &DatabaseConnection,
        parsed: &ItemList,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut query = JournalEntryItemEntity::find();
        if entry_scope(&parsed.entry) {
            let Some(entries) = select_entries(db, &parsed.entry).await? else {
                return Ok(Vec::new());
            };
            let mut entries = entries;
            if let Some(journal_entry_id) = parsed.journal_entry_id {
                find_entry(db, journal_entry_id).await?;
                entries = entries.filter(journal_entry::Column::Id.eq(journal_entry_id));
            }
            let ids: Vec<i64> = entries
                .select_only()
                .column(journal_entry::Column::Id)
                .into_tuple()
                .all(db)
                .await
                .map_err(|error| error.to_string())?;
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            query = query.filter(journal_entry_item::Column::JournalEntryId.is_in(ids));
        } else if let Some(journal_entry_id) = parsed.journal_entry_id {
            find_entry(db, journal_entry_id).await?;
            query = query.filter(journal_entry_item::Column::JournalEntryId.eq(journal_entry_id));
        }
        if let Some(side) = parsed.side {
            query = match side {
                Side::Debit => query.filter(journal_entry_item::Column::Amount.gte(Decimal::ZERO)),
                Side::Credit => query.filter(journal_entry_item::Column::Amount.lt(Decimal::ZERO)),
            };
        }
        let items = query
            .order_by_desc(journal_entry_item::Column::Datetime)
            .order_by_desc(journal_entry_item::Column::Id)
            .limit(parsed.entry.limit.0)
            .all(db)
            .await
            .map_err(|error| error.to_string())?;
        present_items(db, items).await
    }

    pub(super) async fn present_source_docs(
        db: &DatabaseConnection,
        docs: Vec<source_doc::Model>,
    ) -> Result<Vec<serde_json::Value>, String> {
        if docs.is_empty() {
            return Ok(Vec::new());
        }
        let doc_ids = dedup_ids(docs.iter().map(|doc| doc.id));
        let entries = JournalEntryEntity::find()
            .filter(journal_entry::Column::SourceDocId.is_in(doc_ids))
            .order_by_desc(journal_entry::Column::Datetime)
            .order_by_desc(journal_entry::Column::Id)
            .all(db)
            .await
            .map_err(|error| error.to_string())?;
        let presented = present_entries(db, entries).await?;
        let mut by_doc: HashMap<i64, Vec<serde_json::Value>> = HashMap::new();
        for entry in presented {
            let source_doc_id = entry
                .get("source_doc_id")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            by_doc.entry(source_doc_id).or_default().push(entry);
        }
        Ok(docs
            .into_iter()
            .map(|doc| {
                serde_json::json!({
                    "id": doc.id,
                    "source_doc_type": doc.source_doc_type,
                    "source_ref_id": ref_id(doc.source_doc_id),
                    "journal_entries": by_doc.remove(&doc.id).unwrap_or_default(),
                })
            })
            .collect())
    }

    async fn source_docs_for_query(
        db: &DatabaseConnection,
        query: &str,
    ) -> Result<HashSet<i64>, String> {
        let mut ids = HashSet::new();
        let by_type = lariv_core::db::trigram::search::<source_doc::Entity, _>(
            db,
            &[source_doc::Column::SourceDocType],
            query,
            lariv_core::db::trigram::MAX_SEARCH_LIMIT,
        )
        .await
        .map_err(|error| error.to_string())?;
        ids.extend(by_type.into_iter().map(|doc| doc.id));
        let customs = lariv_core::db::trigram::search::<custom_source_doc::Entity, _>(
            db,
            &[custom_source_doc::Column::Title],
            query,
            lariv_core::db::trigram::MAX_SEARCH_LIMIT,
        )
        .await
        .map_err(|error| error.to_string())?;
        let custom_ids = dedup_ids(customs.into_iter().map(|doc| doc.id));
        if !custom_ids.is_empty() {
            let linked = SourceDocEntity::find()
                .filter(source_doc::Column::SourceDocType.eq(CUSTOM_SOURCE_DOC_TYPE))
                .filter(source_doc::Column::SourceDocId.is_in(custom_ids))
                .all(db)
                .await
                .map_err(|error| error.to_string())?;
            ids.extend(linked.into_iter().map(|doc| doc.id));
        }
        if let Ok(id) = query.parse::<i64>()
            && id > 0
        {
            if SourceDocEntity::find_by_id(id)
                .one(db)
                .await
                .map_err(|error| error.to_string())?
                .is_some()
            {
                ids.insert(id);
            }
            let refs = SourceDocEntity::find()
                .filter(source_doc::Column::SourceDocId.eq(id))
                .all(db)
                .await
                .map_err(|error| error.to_string())?;
            ids.extend(refs.into_iter().map(|doc| doc.id));
        }
        Ok(ids)
    }

    pub(super) async fn list_source_docs(
        db: &DatabaseConnection,
        parsed: &SourceList,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut query = SourceDocEntity::find();
        if let Some(id) = parsed.source_doc_id {
            find_source_doc(db, id).await?;
            query = query.filter(source_doc::Column::Id.eq(id));
        }
        if let Some(source_doc_type) = parsed.source_doc_type.as_deref() {
            query = query.filter(source_doc::Column::SourceDocType.eq(source_doc_type));
        }
        if let Some(source_ref_id) = parsed.source_ref_id {
            query = query.filter(source_doc::Column::SourceDocId.eq(source_ref_id));
        }
        if let Some(text) = parsed.query.as_deref() {
            let ids = dedup_ids(source_docs_for_query(db, text).await?);
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            query = query.filter(source_doc::Column::Id.is_in(ids));
        }
        let docs = query
            .order_by_desc(source_doc::Column::Id)
            .limit(parsed.limit.0)
            .all(db)
            .await
            .map_err(|error| error.to_string())?;
        present_source_docs(db, docs).await
    }

    #[cfg(test)]
    mod tests {
        use super::{money, split_amount, transferred_amount};
        use rust_decimal::Decimal;

        #[test]
        fn split_amount_keeps_debit_positive_and_credit_negative() {
            let (debit, credit) = split_amount(Decimal::from(25));
            assert_eq!(debit.as_deref(), Some("25"));
            assert_eq!(credit, None);
            let (debit, credit) = split_amount(Decimal::from(-8));
            assert_eq!(debit, None);
            assert_eq!(credit.as_deref(), Some("8"));
            let (debit, credit) = split_amount(Decimal::ZERO);
            assert_eq!(debit.as_deref(), Some("0"));
            assert_eq!(credit, None);
            assert_eq!(money(Decimal::new(10050, 2)), "100.5");
        }

        #[test]
        fn transferred_amount_is_the_debit_side() {
            assert_eq!(
                transferred_amount([Decimal::from(40), Decimal::from(-40)]),
                "40"
            );
            assert_eq!(
                transferred_amount([Decimal::ZERO, Decimal::from(15), Decimal::from(-15)]),
                "15"
            );
        }
    }
}

#[cfg(all(test, feature = "cap-llm"))]
mod tests {
    use std::sync::Arc;

    use lariv_core::rune_env::{RuneEnvCapability, RuneEnvCtx, RuneEnvRegistrar};
    use lariv_plugin_filesystem::storage::{DynFilestore, UnimplementedFilestore};
    use serde_json::json;

    use super::args::{parse_account_list, parse_bound, parse_entry_list, parse_source_list};
    use super::{
        Hook, get_account, get_journal_entry, get_journal_entry_item, get_source_doc,
        list_journal_entries, list_journal_entry_items, list_source_docs, search_accounts,
    };

    fn ctx<'a>(
        db: &'a sea_orm::DatabaseConnection,
        store: &'a Arc<DynFilestore>,
    ) -> RuneEnvCtx<'a> {
        RuneEnvCtx {
            db,
            store: lariv_plugin_filesystem::storage::erase_store(Arc::clone(store)),
            session_id: None,
        }
    }

    fn value(json_value: serde_json::Value) -> rune::Value {
        lariv_core::rune_env::json_to_rune(json_value).expect("json to rune")
    }

    #[test]
    fn registers_ledger_read_bindings() {
        let mut cap = RuneEnvCapability::new();
        Hook.register_rune_env(&mut cap);
        let names = cap.all_names();
        for name in [
            "get_account",
            "search_accounts",
            "list_accounts",
            "get_journal_entry",
            "list_journal_entries",
            "get_journal_entry_item",
            "list_journal_entry_items",
            "get_source_doc",
            "list_source_docs",
        ] {
            assert!(
                names.iter().any(|registered| registered == name),
                "expected {name} in {names:?}"
            );
        }
    }

    #[test]
    fn reads_reject_bad_arguments_before_a_query() {
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let ctx = ctx(&db, &store);
        let missing = get_account(&ctx, &[]).expect_err("missing");
        assert!(missing.contains("object"));
        let bad_account = get_account(&ctx, &[value(json!({"id": 0}))]).expect_err("id");
        assert!(bad_account.contains("positive"));
        let bad_entry = get_journal_entry(&ctx, &[value(json!({"id": -1}))]).expect_err("entry");
        assert!(bad_entry.contains("positive"));
        let bad_item = get_journal_entry_item(&ctx, &[value(json!({"id": 0}))]).expect_err("item");
        assert!(bad_item.contains("positive"));
        let bad_side =
            list_journal_entry_items(&ctx, &[value(json!({"side": "both"}))]).expect_err("side");
        assert!(bad_side.contains("debit or credit"));
        let bad_doc = get_source_doc(&ctx, &[value(json!({"id": 0}))]).expect_err("doc");
        assert!(bad_doc.contains("positive"));
        let empty_search =
            search_accounts(&ctx, &[value(json!({"query": "  "}))]).expect_err("query");
        assert!(empty_search.contains("query"));
        let bad_type = search_accounts(
            &ctx,
            &[value(json!({"query": "cash", "balance_type": "asset"}))],
        )
        .expect_err("type");
        assert!(bad_type.contains("Debit or Credit"));
        let bad_parent =
            parse_account_list(&[value(json!({"parent_id": -3}))]).expect_err("parent");
        assert!(bad_parent.contains("zero or positive"));
        let bad_from =
            list_journal_entries(&ctx, &[value(json!({"from": "nope"}))]).expect_err("from");
        assert!(bad_from.contains("invalid datetime"));
        let ref_without_type =
            list_journal_entries(&ctx, &[value(json!({"source_ref_id": 4}))]).expect_err("ref");
        assert!(ref_without_type.contains("source_doc_type"));
        let unordered = parse_entry_list(&[value(json!({
            "from": "2026-05-02",
            "to": "2026-05-01",
        }))])
        .expect_err("order");
        assert!(unordered.contains("from is after to"));
        let no_filter = list_source_docs(&ctx, &[value(json!({}))]).expect_err("filter");
        assert!(no_filter.contains("requires"));
        let ref_alone = parse_source_list(&[value(json!({"source_ref_id": 9}))]).expect_err("ref");
        assert!(ref_alone.contains("source_doc_type"));
    }

    #[test]
    fn calendar_dates_cover_a_utc_day() {
        let from = parse_bound("02/05/2026", false).expect("from");
        let to = parse_bound("02/05/2026", true).expect("to");
        assert!(from < to);
        assert_eq!(
            from.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "2026-05-02T00:00:00"
        );
        assert_eq!(
            to.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "2026-05-02T23:59:59"
        );
        let stamp = parse_bound("2026-05-02T15:04:05Z", false).expect("stamp");
        assert_eq!(stamp.to_rfc3339(), "2026-05-02T15:04:05+00:00");
    }
}
