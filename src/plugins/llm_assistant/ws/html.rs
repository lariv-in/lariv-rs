//! HTMX OOB HTML fragments for assistant WebSocket streaming.

use crate::components::markdown::render_markdown;
use crate::components::{HitlApproval, HtmlAttrs, hitl_approval, hitl_resolved};
use crate::plugins::llm_assistant::{
    content::ZWSP,
    context_usage::{ContextUsageView, format_token_count},
    genai::{Content, Role},
};

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn error_oob(message: &str) -> String {
    format!(
        r#"<div id="llm_assistant_errors" hx-swap-oob="true"><div class="alert alert-error text-sm">{}</div></div>
{}"#,
        html_escape(message),
        form_ready_oob()
    )
}

/// Re-enable Send after the assistant turn finishes (or errors).
pub fn form_ready_oob() -> String {
    r#"<button id="llm_assistant_chat_send" hx-swap-oob="true" type="submit" class="btn btn-primary">Send</button>"#
        .to_string()
}

/// Replace Send with Stop while the model / tools are still running.
pub fn form_busy_oob() -> String {
    r#"<button id="llm_assistant_chat_send" hx-swap-oob="true" type="button" class="btn btn-error" data-stop="true">Stop</button>"#
        .to_string()
}

/// Error banner without restoring Send (turn still in flight).
pub fn error_notice_oob(message: &str) -> String {
    format!(
        r#"<div id="llm_assistant_errors" hx-swap-oob="true"><div class="alert alert-error text-sm">{}</div></div>"#,
        html_escape(message)
    )
}

/// Clear the composer after a successful send (HTMX OOB; mirrors hx-on reset).
pub fn clear_message_oob() -> String {
    r#"<textarea id="llm_assistant_chat_message" hx-swap-oob="true" name="message" class="textarea textarea-bordered w-full" rows="3" placeholder="Message…" required></textarea>"#
        .to_string()
}

pub fn clear_errors_oob() -> String {
    r#"<div id="llm_assistant_errors" hx-swap-oob="true"></div>"#.to_string()
}

pub fn user_ack_oob(session_id: i64, user_bubble: &str) -> String {
    format!(
        r#"{}
{}
{}
<input id="llm_assistant_session_id" hx-swap-oob="true" type="hidden" name="session_id" value="{session_id}">
<div id="llm_assistant_transcript" hx-swap-oob="beforeend">{user_bubble}</div>"#,
        clear_errors_oob(),
        clear_message_oob(),
        form_busy_oob(),
    )
}

pub fn session_name_oob(session_name: &str) -> String {
    format!(
        r#"<div id="session-name-container" hx-swap-oob="true" class="text-sm font-semibold truncate max-w-[70%]">{}</div>"#,
        html_escape(session_name)
    )
}

pub fn context_usage_html(usage: ContextUsageView) -> String {
    let pct = usage.percent();
    let used_label = format_token_count(usage.used);
    let max_label = format_token_count(usage.max);
    let title = format!("Context {} / {} tokens ({}%)", usage.used, usage.max, pct);
    format!(
        r#"<div id="llm_assistant_context_usage" class="flex items-center gap-2 min-w-0 flex-1" title="{title}" aria-label="{title}">
<progress class="progress {bar} h-1.5 w-16 sm:w-24 shrink-0" value="{pct}" max="100"></progress>
<span class="text-xs opacity-70 tabular-nums truncate">{used} / {max}</span>
</div>"#,
        title = html_escape(&title),
        bar = usage.progress_class(),
        pct = pct,
        used = html_escape(&used_label),
        max = html_escape(&max_label),
    )
}

pub fn context_usage_oob(usage: ContextUsageView) -> String {
    let mut html = context_usage_html(usage);
    html = html.replacen(
        r#"id="llm_assistant_context_usage""#,
        r#"id="llm_assistant_context_usage" hx-swap-oob="true""#,
        1,
    );
    html
}

pub fn final_assistant_oob(assistant_bubble: &str) -> String {
    format!(
        r#"<div id="llm_assistant_transcript" hx-swap-oob="beforeend">{assistant_bubble}</div>"#
    )
}

/// Collapsed "Chat compacted" group (same details pattern as Tools Called).
pub fn compaction_group_html(summary: &str) -> String {
    let body = render_markdown(summary);
    format!(
        r#"<div class="w-full min-w-0 max-w-full flex flex-col"><details class="text-sm w-full min-w-0"><summary class="text-xs opacity-70 cursor-pointer">Chat compacted</summary><div class="overflow-x-auto p-2 flex flex-col gap-1">{body}</div></details></div>"#
    )
}

pub fn compacted_oob(summary: &str) -> String {
    format!(
        r#"<div id="llm_assistant_transcript" hx-swap-oob="beforeend">{}</div>"#,
        compaction_group_html(summary)
    )
}

/// Replace transcript contents after WS reconnect (keeps the existing element's classes).
pub fn transcript_replace_oob(inner: &str) -> String {
    format!(r#"<div id="llm_assistant_transcript" hx-swap-oob="innerHTML">{inner}</div>"#)
}

pub fn user_bubble_html(content: &Content) -> String {
    let body = parts_visible_html(content, false);
    format!(
        r#"<div class="flex flex-col items-stretch w-full min-w-0 max-w-full"><div class="rounded-lg px-3 py-2 text-sm w-full min-w-0 max-w-full bg-base-content/10 text-base-content">{body}</div></div>"#
    )
}

pub fn assistant_bubble_html(content: &Content) -> String {
    let body = parts_visible_html(content, true);
    format!(
        r#"<div class="flex flex-col items-stretch w-full min-w-0 max-w-full"><div class="px-0 py-1 text-sm w-full min-w-0 max-w-full text-base-content">{body}</div></div>"#
    )
}

/// Visible parts for a model tool-call turn (function calls + any text).
pub fn tool_call_inner_html(content: &Content) -> String {
    parts_visible_html(content, true)
}

/// Function-response parts without an outer wrapper (nested under Tools Called).
pub fn tool_response_inner_html(content: &Content) -> String {
    let mut inner = String::new();
    for p in &content.parts {
        if let Some(fr) = &p.function_response {
            inner.push_str(&function_response_html(fr));
        }
    }
    if inner.is_empty() {
        inner = r#"<span class="opacity-50 text-sm">(empty)</span>"#.into();
    }
    inner
}

/// Collapsed "Tools Called" group for persisted transcript tool sequences.
pub fn working_group_html(inner: &str) -> String {
    format!(
        r#"<div class="w-full min-w-0 max-w-full flex flex-col"><details class="text-sm w-full min-w-0"><summary class="text-xs opacity-70 cursor-pointer">Tools Called</summary><div class="overflow-x-auto p-2 flex flex-col gap-1">{inner}</div></details></div>"#
    )
}

/// First tool activity in a live turn: open Tools Called and seed its body.
pub fn working_open_oob(details_id: &str, body_id: &str, inner: &str) -> String {
    format!(
        r#"<div id="llm_assistant_transcript" hx-swap-oob="beforeend"><div class="w-full min-w-0 max-w-full flex flex-col"><details id="{}" class="text-sm w-full min-w-0" open><summary class="text-xs opacity-70 cursor-pointer">Tools Called</summary><div id="{}" class="overflow-x-auto p-2 flex flex-col gap-1">{}</div></details></div></div>"#,
        html_escape(details_id),
        html_escape(body_id),
        inner
    )
}

/// Append more tool call/response HTML into an existing live Tools Called body.
pub fn working_append_oob(body_id: &str, inner: &str) -> String {
    format!(
        r#"<div id="{}" hx-swap-oob="beforeend">{}</div>"#,
        html_escape(body_id),
        inner
    )
}

/// HITL approval card inner HTML (nested under Tools Called).
pub fn hitl_pending_inner_html(id: &str, name: &str, args: &serde_json::Value) -> String {
    let args_json = serde_json::to_string_pretty(args).unwrap_or_else(|_| args.to_string());
    let run_attrs = HtmlAttrs::new()
        .set("data-hitl-id", id)
        .set("data-hitl-approve", "true");
    let deny_attrs = HtmlAttrs::new()
        .set("data-hitl-id", id)
        .set("data-hitl-deny", "true");
    hitl_approval(HitlApproval {
        request_id: id,
        function_name: name,
        args_json: &args_json,
        run_attrs,
        deny_attrs,
    })
    .into_string()
}

/// OOB replace of a HITL card after Run / Deny.
pub fn hitl_resolved_oob(id: &str, name: &str, approved: bool) -> String {
    let mut markup = hitl_resolved(id, name, approved).into_string();
    if let Some(pos) = markup.find('>') {
        markup.insert_str(pos, r#" hx-swap-oob="outerHTML""#);
    }
    markup
}

/// Collapse a live Tools Called group when the assistant text reply is sent.
///
/// Targets `#llm_assistant_working_close` (always present in the chat shell); the
/// client removes `open` from the details id carried in `data-details-id`.
pub fn working_close_oob(details_id: &str) -> String {
    format!(
        r#"<span id="llm_assistant_working_close" hx-swap-oob="true" hidden data-details-id="{}"></span>"#,
        html_escape(details_id)
    )
}

fn function_call_html(fc: &crate::plugins::llm_assistant::genai::FunctionCall) -> String {
    let title = if fc.name.is_empty() {
        "Function call".to_string()
    } else {
        format!("Function call: {}", html_escape(&fc.name))
    };
    let mut b = String::new();
    b.push_str(&format!(
        r#"<details class="text-sm w-full"><summary class="text-xs opacity-70 cursor-pointer">{title}</summary><div class="overflow-x-auto p-2"><div class="assistant-part assistant-part-fn-call text-sm">"#
    ));
    if !fc.id.is_empty() {
        b.push_str(&format!(
            r#"<div class="text-xs opacity-70">ID <code>{}</code></div>"#,
            html_escape(&fc.id)
        ));
    }
    if let Some(wc) = fc.will_continue {
        b.push_str(&format!(
            r#"<div class="text-xs">willContinue: <span class="font-mono">{wc}</span></div>"#
        ));
    }
    if fc
        .args
        .as_ref()
        .is_some_and(|v| !v.is_null() && !v.as_object().is_some_and(|o| o.is_empty()))
    {
        b.push_str(r#"<div class="text-xs font-medium opacity-70">Arguments</div>"#);
        b.push_str(&map_html(fc.args.as_ref().expect("checked above")));
    } else if let Some(args) = &fc.args {
        // Show non-object / empty-object args too so the call is never silent.
        b.push_str(r#"<div class="text-xs font-medium opacity-70">Arguments</div>"#);
        b.push_str(&map_html(args));
    } else {
        b.push_str(r#"<div class="text-xs opacity-50">No arguments</div>"#);
    }
    b.push_str("</div></div></details>");
    b
}

fn function_response_html(fr: &crate::plugins::llm_assistant::genai::FunctionResponse) -> String {
    let title = if fr.name.is_empty() {
        "Function response".to_string()
    } else {
        format!("Function response: {}", html_escape(&fr.name))
    };
    let mut b = String::new();
    b.push_str(&format!(
        r#"<details class="text-sm w-full"><summary class="text-xs opacity-70 cursor-pointer">{title}</summary><div class="overflow-x-auto"><div class="assistant-part assistant-part-fn-resp text-sm">"#
    ));
    if !fr.function_response_id.is_empty() {
        b.push_str(&format!(
            r#"<div class="text-xs opacity-70">Call ID <code>{}</code></div>"#,
            html_escape(&fr.function_response_id)
        ));
    }
    if let Some(resp) = &fr.response {
        b.push_str(r#"<div class="text-xs font-medium opacity-70">Response</div>"#);
        b.push_str(&map_html(resp));
    }
    b.push_str("</div></div></details>");
    b
}

fn map_html(v: &serde_json::Value) -> String {
    match serde_json::to_string_pretty(v) {
        Ok(s) => format!(
            r#"<pre class="text-xs whitespace-pre-wrap font-mono text-base-content border border-base-content/15 rounded p-1 m-0">{}</pre>"#,
            html_escape(&s)
        ),
        Err(_) => String::new(),
    }
}

fn parts_visible_html(content: &Content, markdown: bool) -> String {
    let mut out = Vec::new();
    let mut text_buf = String::new();

    let flush_text = |buf: &mut String, out: &mut Vec<String>, markdown: bool| {
        if buf.is_empty() {
            return;
        }
        if markdown {
            out.push(render_markdown(buf));
        } else {
            out.push(html_escape(buf).replace('\n', "<br>"));
        }
        buf.clear();
    };

    for p in &content.parts {
        if let Some(t) = &p.text {
            if t != ZWSP && !t.is_empty() {
                text_buf.push_str(t);
            }
        }
        let has_attachment = p.inline_data.is_some();
        let has_fc = p.function_call.is_some();
        if has_attachment || has_fc {
            flush_text(&mut text_buf, &mut out, markdown);
            if p.inline_data.is_some() {
                let name = if p.display_name.is_empty() {
                    "attachment"
                } else {
                    p.display_name.as_str()
                };
                out.push(format!(
                    r#"<div class="text-xs opacity-80 mt-1">[{}]</div>"#,
                    html_escape(name)
                ));
            }
            if let Some(fc) = &p.function_call {
                out.push(function_call_html(fc));
            }
        }
    }
    flush_text(&mut text_buf, &mut out, markdown);

    if out.is_empty() && content.role == Role::Model {
        out.push("<em>…</em>".into());
    }
    out.join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::llm_assistant::context_usage::ContextUsageView;

    #[test]
    fn compaction_group_is_details_dropdown() {
        let html = compaction_group_html("Prior goals.");
        assert!(html.contains("Chat compacted"));
        assert!(html.contains("<details"));
        assert!(html.contains("Prior goals."));
    }

    #[test]
    fn compacted_oob_appends_to_transcript() {
        let html = compacted_oob("Summary text.");
        assert!(html.contains(r#"id="llm_assistant_transcript""#));
        assert!(html.contains(r#"hx-swap-oob="beforeend""#));
        assert!(html.contains("Chat compacted"));
        assert!(!html.contains("llm_assistant_chat_send"));
    }

    #[test]
    fn form_busy_is_stop_button() {
        let html = form_busy_oob();
        assert!(html.contains(r#"id="llm_assistant_chat_send""#));
        assert!(html.contains(r#"type="button""#));
        assert!(html.contains(r#"data-stop="true""#));
        assert!(html.contains(">Stop</button>"));
        assert!(!html.contains("disabled"));
        assert!(!html.contains(">Send</button>"));
    }

    #[test]
    fn form_ready_is_send_submit() {
        let html = form_ready_oob();
        assert!(html.contains(r#"id="llm_assistant_chat_send""#));
        assert!(html.contains(r#"type="submit""#));
        assert!(html.contains(">Send</button>"));
        assert!(!html.contains("data-stop"));
        assert!(!html.contains(">Stop</button>"));
    }

    #[test]
    fn working_open_includes_details_id_and_open() {
        let html = working_open_oob(
            "llm_assistant_working_details_1",
            "llm_assistant_working_body_1",
            "<p>x</p>",
        );
        assert!(html.contains(r#"id="llm_assistant_working_details_1""#));
        assert!(html.contains(r#"id="llm_assistant_working_body_1""#));
        assert!(html.contains(" open>"));
        assert!(html.contains("Tools Called"));
    }

    #[test]
    fn working_close_carries_details_id() {
        let html = working_close_oob("llm_assistant_working_details_1");
        assert!(html.contains(r#"id="llm_assistant_working_close""#));
        assert!(html.contains(r#"data-details-id="llm_assistant_working_details_1""#));
        assert!(html.contains(r#"hx-swap-oob="true""#));
    }

    #[test]
    fn context_usage_meter_shows_fill() {
        let html = context_usage_html(ContextUsageView::new(12_450, 1_048_576));
        assert!(html.contains(r#"id="llm_assistant_context_usage""#));
        assert!(html.contains("12k / 1.0M"));
        assert!(html.contains("progress-success"));
        assert!(html.contains(r#"value="1""#));
        let oob = context_usage_oob(ContextUsageView::new(950_000, 1_000_000));
        assert!(oob.contains(r#"hx-swap-oob="true""#));
        assert!(oob.contains("progress-error"));
        assert!(oob.contains("950k / 1M"));
    }
}
