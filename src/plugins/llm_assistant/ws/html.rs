//! HTMX OOB HTML fragments for assistant WebSocket streaming.

use crate::components::field::render_markdown;
use crate::plugins::llm_assistant::{
    content::ZWSP,
    genai::{Content, ROLE_MODEL},
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

/// Keep Send disabled while the model / tools are still running.
pub fn form_busy_oob() -> String {
    r#"<button id="llm_assistant_chat_send" hx-swap-oob="true" type="submit" class="btn btn-primary" disabled>Send</button>"#
        .to_string()
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

pub fn final_assistant_oob(assistant_bubble: &str) -> String {
    format!(
        r#"<div id="llm_assistant_transcript" hx-swap-oob="beforeend">{assistant_bubble}</div>
{}"#,
        form_ready_oob()
    )
}

pub fn user_bubble_html(content: &Content) -> String {
    let body = parts_visible_html(content, false);
    format!(
        r#"<div class="flex flex-col items-stretch w-full"><div class="rounded-lg px-3 py-2 text-sm w-full bg-base-content/10 text-base-content">{body}</div></div>"#
    )
}

pub fn assistant_bubble_html(content: &Content) -> String {
    let body = parts_visible_html(content, true);
    format!(
        r#"<div class="flex flex-col items-stretch w-full"><div class="px-0 py-1 text-sm w-full text-base-content prose prose-sm max-w-none">{body}</div></div>"#
    )
}

/// Append an intermediate assistant turn (e.g. tool call with args) without re-enabling Send.
pub fn assistant_append_oob(assistant_bubble: &str) -> String {
    format!(
        r#"<div id="llm_assistant_transcript" hx-swap-oob="beforeend">{assistant_bubble}</div>"#
    )
}

pub fn tool_oob(tool_bubble: &str) -> String {
    format!(r#"<div id="llm_assistant_transcript" hx-swap-oob="beforeend">{tool_bubble}</div>"#)
}

/// Collapsible tool-execution bubble for functionResponse user turns.
pub fn tool_bubble_html(content: &Content) -> String {
    let mut inner = String::new();
    for p in &content.parts {
        if let Some(fr) = &p.function_response {
            inner.push_str(&function_response_html(fr));
        }
    }
    if inner.is_empty() {
        inner = r#"<span class="opacity-50 text-sm">(empty)</span>"#.into();
    }
    format!(
        r#"<div class="w-full flex flex-col"><details class="text-sm w-full"><summary class="text-xs opacity-70 cursor-pointer">Tool Execution</summary><div class="overflow-x-auto">{inner}</div></details></div>"#
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
        r#"<details class="text-sm w-full"><summary class="text-xs opacity-70 cursor-pointer">{title}</summary><div class="overflow-x-auto"><div class="assistant-part assistant-part-fn-call text-sm">"#
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
            if let Some(blob) = &p.inline_data {
                let name = if blob.display_name.is_empty() {
                    "attachment"
                } else {
                    blob.display_name.as_str()
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

    if out.is_empty() && content.role.eq_ignore_ascii_case(ROLE_MODEL) {
        out.push("<em>…</em>".into());
    }
    out.join("")
}
