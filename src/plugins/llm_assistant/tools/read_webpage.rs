//! `read_webpage` — fetch a public http(s) URL and return readable text.

use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    llm_tools::{LlmTool, ToolCtx},
    plugins::llm_assistant::{config::WEBPAGE_TEXT_CHAR_LIMIT, genai::FunctionDeclaration},
};

use super::http_fetch::{FetchOptions, fetch_public_url};

const FETCH_TIMEOUT_SECS: u64 = 20;
const MAX_BYTES: usize = 2 * 1024 * 1024;
const USER_AGENT: &str = "LarivAssistant/0.1 (read_webpage)";
const ACCEPT: &str = "text/html,application/xhtml+xml,text/plain;q=0.9,*/*;q=0.8";

const SKIP_TAGS: &[&str] = &[
    "script", "style", "noscript", "svg", "iframe", "template", "head",
];

const BLOCK_TAGS: &[&str] = &[
    "p",
    "div",
    "br",
    "hr",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "li",
    "ul",
    "ol",
    "tr",
    "table",
    "blockquote",
    "section",
    "article",
    "header",
    "footer",
    "main",
    "pre",
    "dd",
    "dt",
    "figure",
    "figcaption",
];

pub struct ReadWebpageTool;

#[derive(Debug, Deserialize, Default)]
struct Args {
    #[serde(default)]
    url: String,
    #[serde(default)]
    max_chars: i32,
}

#[async_trait]
impl LlmTool for ReadWebpageTool {
    fn name(&self) -> &str {
        "read_webpage"
    }

    fn declaration(&self) -> FunctionDeclaration {
        FunctionDeclaration {
            name: "read_webpage".into(),
            description: "Fetch a public http(s) URL and return the page title and readable text. \
                Use after google_search when you need the full content of a specific page. \
                Private, local, and credentialed URLs are rejected."
                .into(),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "http(s) URL to fetch" },
                    "max_chars": {
                        "type": "integer",
                        "description": format!(
                            "Max characters of extracted text (default {WEBPAGE_TEXT_CHAR_LIMIT})"
                        )
                    }
                },
                "required": ["url"]
            })),
        }
    }

    async fn run(&self, _ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String> {
        let parsed: Args = serde_json::from_value(args).unwrap_or_default();
        let url = parsed.url.trim();
        if url.is_empty() {
            return Err("read_webpage: url is required".into());
        }
        let mut max_chars = parsed.max_chars;
        if max_chars <= 0 {
            max_chars = WEBPAGE_TEXT_CHAR_LIMIT as i32;
        }
        let cap = usize::try_from(max_chars).unwrap_or(WEBPAGE_TEXT_CHAR_LIMIT);
        let cap = cap.min(WEBPAGE_TEXT_CHAR_LIMIT);

        let (final_url, title, text) = fetch_page(url).await?;
        let (text, truncated) = truncate_chars(&text, cap);
        Ok(json!({
            "url": final_url,
            "title": title,
            "text": text,
            "truncated": truncated,
        }))
    }
}

async fn fetch_page(url_str: &str) -> Result<(String, String, String), String> {
    let fetched = fetch_public_url(
        url_str,
        &FetchOptions {
            user_agent: USER_AGENT,
            accept: ACCEPT,
            max_bytes: MAX_BYTES,
            timeout: Duration::from_secs(FETCH_TIMEOUT_SECS),
            error_prefix: "read_webpage",
        },
    )
    .await?;
    let raw = String::from_utf8_lossy(&fetched.body);
    let ctype = fetched.content_type.as_str();
    let (title, readable) = if is_html(ctype, &raw) {
        html_to_readable(&raw)
    } else if is_plain(ctype) || ctype.is_empty() {
        (String::new(), normalize_ws(&raw))
    } else {
        return Err(format!("read_webpage: unsupported content type ({ctype})"));
    };
    if title.is_empty() && readable.is_empty() {
        return Err("read_webpage: no readable text".into());
    }
    Ok((fetched.url, title, readable))
}

fn is_html(ctype: &str, body: &str) -> bool {
    if ctype.contains("text/html") || ctype.contains("application/xhtml+xml") {
        return true;
    }
    let trimmed = body.trim_start();
    trimmed.starts_with("<!DOCTYPE html")
        || trimmed.starts_with("<!doctype html")
        || trimmed.starts_with("<html")
        || trimmed.starts_with("<HTML")
}

fn is_plain(ctype: &str) -> bool {
    ctype.contains("text/plain")
}

fn html_to_readable(html: &str) -> (String, String) {
    let title = extract_title(html);
    let text = render_html_text(html);
    (title, text)
}

fn extract_title(html: &str) -> String {
    let Some(start_tag) = find_ci(html, "<title") else {
        return String::new();
    };
    let from_tag = html.get(start_tag..).unwrap_or("");
    let Some(gt) = find_tag_end(from_tag) else {
        return String::new();
    };
    let content_start = start_tag.saturating_add(gt).saturating_add(1);
    let from_content = html.get(content_start..).unwrap_or("");
    let Some(end) = find_ci(from_content, "</title>") else {
        return String::new();
    };
    let raw = from_content.get(..end).unwrap_or("");
    collapse_line(&decode_entities(raw))
}

fn render_html_text(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(lt) = rest.find('<') {
        push_decoded_text(&mut out, rest.get(..lt).unwrap_or(""));
        rest = rest.get(lt..).unwrap_or("");
        if rest.is_empty() {
            break;
        }

        if starts_with_ci(rest, "<!--") {
            let skip = find_ci(rest, "-->")
                .map(|i| i.saturating_add(3))
                .unwrap_or(rest.len());
            rest = rest.get(skip..).unwrap_or("");
            continue;
        }
        if starts_with_ci(rest, "<?") {
            let skip = rest
                .find("?>")
                .map(|i| i.saturating_add(2))
                .unwrap_or(rest.len());
            rest = rest.get(skip..).unwrap_or("");
            continue;
        }
        if starts_with_ci(rest, "<!") {
            let skip = rest
                .find('>')
                .map(|i| i.saturating_add(1))
                .unwrap_or(rest.len());
            rest = rest.get(skip..).unwrap_or("");
            continue;
        }

        let Some(gt) = find_tag_end(rest) else {
            break;
        };
        let tag = rest.get(..=gt).unwrap_or("");
        rest = rest.get(gt.saturating_add(1)..).unwrap_or("");
        let (name, is_close, is_self_close) = parse_tag(tag);

        if !is_close && SKIP_TAGS.iter().any(|t| *t == name) {
            if !is_self_close {
                let close = format!("</{name}>");
                let skip = find_ci(rest, &close)
                    .map(|i| i.saturating_add(close.len()))
                    .unwrap_or(rest.len());
                rest = rest.get(skip..).unwrap_or("");
            }
            continue;
        }

        if BLOCK_TAGS.iter().any(|t| *t == name) {
            push_nl(&mut out);
        }
    }
    push_decoded_text(&mut out, rest);
    normalize_ws(&out)
}

fn parse_tag(tag: &str) -> (String, bool, bool) {
    let inner = tag
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim();
    let is_close = inner.starts_with('/');
    let inner = inner.trim_start_matches('/').trim();
    let is_self_close = inner.ends_with('/');
    let inner = inner.trim_end_matches('/').trim();
    let name = inner
        .split(|c: char| c.is_whitespace() || c == '/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    (name, is_close, is_self_close)
}

fn find_tag_end(s: &str) -> Option<usize> {
    let mut quote: Option<u8> = None;
    for (i, b) in s.bytes().enumerate() {
        if let Some(q) = quote {
            if b == q {
                quote = None;
            }
            continue;
        }
        match b {
            b'"' | b'\'' => quote = Some(b),
            b'>' => return Some(i),
            _ => {}
        }
    }
    None
}

fn starts_with_ci(s: &str, prefix: &str) -> bool {
    s.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

fn find_ci(hay: &str, needle: &str) -> Option<usize> {
    let n = needle.len();
    if n == 0 {
        return Some(0);
    }
    hay.as_bytes()
        .windows(n)
        .position(|w| w.eq_ignore_ascii_case(needle.as_bytes()))
        .filter(|&i| hay.is_char_boundary(i))
}

fn push_decoded_text(out: &mut String, raw: &str) {
    if raw.is_empty() {
        return;
    }
    out.push_str(&decode_entities(raw));
}

fn push_nl(out: &mut String) {
    if !out.ends_with('\n') {
        out.push('\n');
    }
}

fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(off) = rest.find('&') {
        out.push_str(rest.get(..off).unwrap_or(""));
        rest = rest.get(off..).unwrap_or("");
        let Some(semi) = rest.find(';') else {
            out.push_str(rest);
            return out;
        };
        if semi > 32 {
            out.push('&');
            rest = rest.get(1..).unwrap_or("");
            continue;
        }
        let ent = rest.get(..=semi).unwrap_or("");
        match entity_char(ent) {
            Some(ch) => out.push(ch),
            None => out.push_str(ent),
        }
        rest = rest.get(semi.saturating_add(1)..).unwrap_or("");
    }
    out.push_str(rest);
    out
}

fn entity_char(ent: &str) -> Option<char> {
    let named = match ent {
        "&amp;" => Some('&'),
        "&lt;" => Some('<'),
        "&gt;" => Some('>'),
        "&quot;" => Some('"'),
        "&apos;" | "&#39;" => Some('\''),
        "&nbsp;" => Some(' '),
        _ => None,
    };
    if named.is_some() {
        return named;
    }
    let inner = ent
        .strip_prefix("&#x")
        .or_else(|| ent.strip_prefix("&#X"))
        .map(|s| (s, 16))
        .or_else(|| ent.strip_prefix("&#").map(|s| (s, 10)))?;
    let (digits, radix) = inner;
    let digits = digits.strip_suffix(';')?;
    let n = u32::from_str_radix(digits, radix).ok()?;
    char::from_u32(n).filter(|c| *c == '\t' || *c == '\n' || !c.is_control())
}

fn collapse_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_ws(s: &str) -> String {
    let mut out = String::new();
    let mut blank = 0u8;
    for line in s.lines() {
        let line = collapse_line(line);
        if line.is_empty() {
            blank = blank.saturating_add(1);
            if blank <= 1 && !out.is_empty() {
                out.push('\n');
            }
            continue;
        }
        blank = 0;
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&line);
    }
    out
}

fn truncate_chars(s: &str, max: usize) -> (String, bool) {
    if s.chars().count() <= max {
        return (s.to_string(), false);
    }
    (s.chars().take(max).collect(), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_strips_script_style_and_decodes_entities() {
        let html = r#"<html>
            <head><title>Hello &amp; Co</title>
            <script>var secret = 1;</script>
            <style>p { color: red; }</style>
            </head>
            <body>
            <h1>Welcome</h1>
            <p>This is <b>bold</b> and a link.</p>
            <!-- hidden -->
            </body>
            </html>"#;
        let (title, text) = html_to_readable(html);
        assert_eq!(title, "Hello & Co");
        assert!(text.contains("Welcome"));
        assert!(text.contains("This is bold and a link."));
        assert!(!text.contains("secret"));
        assert!(!text.contains("color: red"));
        assert!(!text.contains("hidden"));
    }

    #[test]
    fn truncate_marks_overflow() {
        let (out, truncated) = truncate_chars("abcdef", 3);
        assert_eq!(out, "abc");
        assert!(truncated);
        let (out, truncated) = truncate_chars("ab", 3);
        assert_eq!(out, "ab");
        assert!(!truncated);
    }
}
