//! `google_search` — Google Programmable Search (Custom Search JSON API).

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::llm_assistant::{
        config::GOOGLE_SEARCH_RESULT_LIMIT_CAP,
        genai::FunctionDeclaration,
    },
};

const CSE_ENDPOINT: &str = "https://www.googleapis.com/customsearch/v1";
const PAGE_SIZE: i32 = 10;
const MAX_PAGES: i32 = 2;

pub struct GoogleSearchTool;

#[derive(Debug, Deserialize, Default)]
struct Args {
    #[serde(default)]
    query: String,
    #[serde(default)]
    limit: i32,
}

#[derive(Debug, Deserialize)]
struct CseResponse {
    #[serde(default)]
    items: Vec<CseItem>,
}

#[derive(Debug, Deserialize)]
struct CseItem {
    #[serde(default)]
    title: String,
    #[serde(default)]
    link: String,
    #[serde(default)]
    snippet: String,
}

#[async_trait]
impl LlmTool for GoogleSearchTool {
    fn name(&self) -> &str {
        "google_search"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "google_search".into(),
            description: "Search the public web via Google Custom Search (configured in Lariv). Use when you need to search or verify details on the web.".into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" },
                    "limit": { "type": "integer", "description": "Max results (default 8)" }
                },
                "required": ["query"]
            })),
        }
    }

    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: Args = serde_json::from_value(args).unwrap_or_default();
        let hits = run_cse(ctx, &parsed.query, parsed.limit).await?;
        Ok(json!({ "hits": hits }))
    }
}

async fn run_cse(ctx: &ToolCtx<'_>, query: &str, mut limit: i32) -> Result<Vec<Value>, String> {
    let key = ctx.cse_api_key.trim();
    let cx = ctx.cse_cx.trim();
    if key.is_empty() || cx.is_empty() {
        return Err(
            "google_search: configure [llm_assistant] cseApiKey and cseCx".into(),
        );
    }
    let q = query.trim();
    if q.is_empty() {
        return Err("google_search: empty query".into());
    }
    if limit <= 0 {
        limit = 8;
    }
    if limit > GOOGLE_SEARCH_RESULT_LIMIT_CAP {
        limit = GOOGLE_SEARCH_RESULT_LIMIT_CAP;
    }

    let client = reqwest::Client::new();
    let mut hits = Vec::new();
    let mut max_pages = (limit + PAGE_SIZE - 1) / PAGE_SIZE;
    if max_pages < 1 {
        max_pages = 1;
    }
    if max_pages > MAX_PAGES {
        max_pages = MAX_PAGES;
    }

    for page in 0..max_pages {
        if hits.len() as i32 >= limit {
            break;
        }
        let need = limit - hits.len() as i32;
        let n = need.min(PAGE_SIZE);
        let start = 1 + page * PAGE_SIZE;
        let url = format!(
            "{CSE_ENDPOINT}?key={}&cx={}&q={}&num={n}&start={start}",
            urlencoding_encode(key),
            urlencoding_encode(cx),
            urlencoding_encode(q),
        );
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("google_search: {e}"))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("google_search: HTTP status {}", status.as_u16()));
        }
        let parsed: CseResponse =
            serde_json::from_str(&body).map_err(|e| format!("google_search: response json: {e}"))?;
        if parsed.items.is_empty() {
            break;
        }
        for it in parsed.items {
            let link = it.link.trim();
            if link.is_empty() {
                continue;
            }
            hits.push(json!({
                "title": it.title.trim(),
                "link": link,
                "snippet": it.snippet.trim(),
            }));
            if hits.len() as i32 >= limit {
                break;
            }
        }
    }
    Ok(hits)
}

fn urlencoding_encode(s: &str) -> String {
    // Minimal query encoding for CSE params.
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
