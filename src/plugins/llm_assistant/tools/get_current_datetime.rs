//! `get_current_datetime` — current UTC instant, optionally in an IANA timezone.

use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use chrono_tz::Tz;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::llm_assistant::genai::FunctionDeclaration,
};

pub struct GetCurrentDatetimeTool;

#[derive(Debug, Deserialize, Default)]
struct Args {
    #[serde(default)]
    timezone: String,
}

#[async_trait]
impl LlmTool for GetCurrentDatetimeTool {
    fn name(&self) -> &str {
        "get_current_datetime"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "get_current_datetime".into(),
            description:
                "Get the current date and time. Always call this instead of guessing the date \
                or clock. Omit timezone for UTC; pass an IANA name (for example Asia/Kolkata) \
                for that zone's local date, time, and offset."
                    .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "timezone": {
                        "type": "string",
                        "description": "IANA timezone (e.g. UTC, Asia/Kolkata, America/New_York). Defaults to UTC."
                    }
                }
            })),
        }
    }

    async fn run(&self, _ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: Args = serde_json::from_value(args).unwrap_or_default();
        snapshot(Utc::now(), &parsed.timezone)
    }
}

fn parse_tz(timezone: &str) -> Result<Tz, String> {
    let timezone = timezone.trim();
    if timezone.is_empty() {
        return Ok(chrono_tz::UTC);
    }
    timezone
        .parse::<Tz>()
        .map_err(|_| format!("unknown timezone {timezone:?}"))
}

fn snapshot(now: DateTime<Utc>, timezone: &str) -> Result<Value, String> {
    let tz = parse_tz(timezone)?;
    let local = now.with_timezone(&tz);
    Ok(json!({
        "utc": now.to_rfc3339_opts(SecondsFormat::Secs, true),
        "unix": now.timestamp(),
        "timezone": tz.name(),
        "offset": local.format("%:z").to_string(),
        "local": local.to_rfc3339_opts(SecondsFormat::Secs, false),
        "date": local.format("%Y-%m-%d").to_string(),
        "time": local.format("%H:%M:%S").to_string(),
        "weekday": local.format("%A").to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use chrono::TimeZone;

    use crate::{
        llm_tools::ToolCtx,
        plugins::filesystem::storage::{DynFilestore, UnimplementedFilestore},
        rune_env::RuneEnvCapability,
    };

    fn ctx<'a>(
        db: &'a sea_orm::DatabaseConnection,
        store: Arc<DynFilestore>,
        rune_env: &'a RuneEnvCapability,
    ) -> ToolCtx<'a> {
        ToolCtx {
            db,
            store,
            cse_api_key: "",
            cse_cx: "",
            rune_env,
            hitl: None,
            hitl_gate: None,
            session_id: None,
            genai: None,
        }
    }

    #[test]
    fn declaration_names_the_tool() {
        let decl = GetCurrentDatetimeTool.declaration();
        assert_eq!(decl.name, "get_current_datetime");
        assert!(decl.description.contains("IANA"));
    }

    #[test]
    fn kolkata_converts_from_utc() {
        let now = Utc.with_ymd_and_hms(2026, 9, 9, 6, 55, 12).unwrap();
        let out = snapshot(now, "Asia/Kolkata").unwrap();
        assert_eq!(out["utc"], "2026-09-09T06:55:12Z");
        assert_eq!(out["unix"], 1788936912i64);
        assert_eq!(out["timezone"], "Asia/Kolkata");
        assert_eq!(out["offset"], "+05:30");
        assert_eq!(out["local"], "2026-09-09T12:25:12+05:30");
        assert_eq!(out["date"], "2026-09-09");
        assert_eq!(out["time"], "12:25:12");
        assert_eq!(out["weekday"], "Wednesday");
    }

    #[test]
    fn empty_timezone_is_utc() {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let out = snapshot(now, "  ").unwrap();
        assert_eq!(out["timezone"], "UTC");
        assert_eq!(out["offset"], "+00:00");
        assert_eq!(out["local"], "2026-01-01T00:00:00+00:00");
        assert_eq!(out["weekday"], "Thursday");
    }

    #[test]
    fn unknown_timezone_errors() {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let err = snapshot(now, "Not/AZone").unwrap_err();
        assert!(err.contains("unknown timezone"), "{err}");
    }

    #[tokio::test]
    async fn run_returns_utc_when_timezone_omitted() {
        let cap = RuneEnvCapability::new();
        let db = sea_orm::DatabaseConnection::default();
        let store: Arc<DynFilestore> = Arc::new(UnimplementedFilestore);
        let out = GetCurrentDatetimeTool
            .run(&ctx(&db, store, &cap), json!({}))
            .await
            .unwrap();
        assert_eq!(out["timezone"], "UTC");
        assert!(out["utc"].as_str().unwrap().ends_with('Z'));
        assert!(out["unix"].as_i64().is_some());
    }
}
