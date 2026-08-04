//! HTMX OOB HTML fragments for assistant WebSocket streaming.

use crate::components::field::render_markdown;
use crate::plugins::llm_assistant::{
    content::ZWSP,
    genai::{Content, ROLE_MODEL},
};

const STREAM_CLASS: &str =
    "w-full max-w-2xl mx-auto mb-4 min-h-[1.5rem] border border-dashed border-base-300 rounded-lg p-4 text-sm";

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

pub fn form_ready_oob() -> String {
    r#"<button id="llm_assistant_chat_send" hx-swap-oob="true" type="submit" class="btn btn-primary self-end">Send</button>"#
        .to_string()
}

pub fn clear_errors_oob() -> String {
    r#"<div id="llm_assistant_errors" hx-swap-oob="true"></div>"#.to_string()
}

pub fn user_ack_oob(session_id: i64, user_bubble: &str) -> String {
    format!(
        r#"{}
<input id="llm_assistant_session_id" hx-swap-oob="true" type="hidden" name="session_id" value="{session_id}">
<div id="llm_assistant_transcript" hx-swap-oob="beforeend">{user_bubble}</div>"#,
        clear_errors_oob(),
    )
}

pub fn stream_oob(inner_html: &str) -> String {
    format!(
        r#"<div id="llm_assistant_stream" hx-swap-oob="true" class="{STREAM_CLASS}">{inner_html}</div>"#
    )
}

pub fn clear_stream_oob() -> String {
    format!(r#"<div id="llm_assistant_stream" hx-swap-oob="true" class="{STREAM_CLASS}"></div>"#)
}

pub fn final_assistant_oob(assistant_bubble: &str) -> String {
    format!(
        r#"{}
<div id="llm_assistant_transcript" hx-swap-oob="beforeend">{assistant_bubble}</div>
{}"#,
        clear_stream_oob(),
        form_ready_oob()
    )
}

pub fn user_bubble_html(content: &Content) -> String {
    let body = parts_visible_html(content, false);
    format!(
        r#"<div class="flex flex-col items-end mb-3 w-full max-w-2xl mx-auto"><div class="text-xs opacity-60 mb-1">user</div><div class="rounded-lg px-3 py-2 text-sm bg-primary text-primary-content max-w-[90%]">{body}</div></div>"#
    )
}

pub fn assistant_bubble_html(content: &Content) -> String {
    let body = parts_visible_html(content, true);
    format!(
        r#"<div class="flex flex-col items-start mb-3 w-full max-w-2xl mx-auto"><div class="text-xs opacity-60 mb-1">assistant</div><div class="rounded-lg px-3 py-2 text-sm bg-base-200 max-w-[90%] prose prose-sm">{body}</div></div>"#
    )
}

/// Stream UI shows text parts only (functionCall parts are handled after the round).
pub fn stream_inner_html(content: &Content) -> String {
    let mut texts = Vec::new();
    for p in &content.parts {
        if let Some(t) = &p.text {
            if t == ZWSP || t.is_empty() {
                continue;
            }
            texts.push(render_markdown(t));
        }
    }
    texts.join("")
}

pub fn tool_oob(tool_bubble: &str) -> String {
    format!(
        r#"{}
<div id="llm_assistant_transcript" hx-swap-oob="beforeend">{tool_bubble}</div>"#,
        clear_stream_oob(),
    )
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
        r#"<div class="w-full max-w-2xl mx-auto mb-3 flex flex-col"><details class="collapse text-sm w-fit"><summary class="text-xs opacity-70 cursor-pointer p-0">Tool Execution</summary><div class="collapse-content p-3 pt-0 overflow-x-auto">{inner}</div></details></div>"#
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
        r#"<details class="collapse text-sm w-fit"><summary class="text-xs text-gray-300 cursor-pointer p-0">{title}</summary><div class="collapse-content p-3 pt-0 overflow-x-auto"><div class="assistant-part assistant-part-fn-call text-sm space-y-2 mt-2">"#
    ));
    if !fc.id.is_empty() {
        b.push_str(&format!(
            r#"<div class="mb-1 text-xs opacity-70">ID <code>{}</code></div>"#,
            html_escape(&fc.id)
        ));
    }
    if let Some(wc) = fc.will_continue {
        b.push_str(&format!(
            r#"<div class="mb-1 text-xs">willContinue: <span class="font-mono">{wc}</span></div>"#
        ));
    }
    if fc
        .args
        .as_ref()
        .is_some_and(|v| !v.is_null() && !v.as_object().is_some_and(|o| o.is_empty()))
    {
        b.push_str(r#"<div class="text-xs font-medium opacity-70 mb-1">Arguments</div>"#);
        b.push_str(&map_html(fc.args.as_ref().expect("checked above")));
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
        r#"<details class="collapse text-sm max-w-full my-2"><summary class="text-xs opacity-70 cursor-pointer p-0">{title}</summary><div class="collapse-content p-3 pt-0 overflow-x-auto"><div class="assistant-part assistant-part-fn-resp text-sm space-y-2 mt-2">"#
    ));
    if !fr.function_response_id.is_empty() {
        b.push_str(&format!(
            r#"<div class="mb-1 text-xs opacity-70">Call ID <code>{}</code></div>"#,
            html_escape(&fr.function_response_id)
        ));
    }
    if let Some(resp) = &fr.response {
        b.push_str(r#"<div class="text-xs font-medium opacity-70 mb-1">Response</div>"#);
        b.push_str(&map_html(resp));
    }
    b.push_str("</div></div></details>");
    b
}

fn map_html(v: &serde_json::Value) -> String {
    match serde_json::to_string_pretty(v) {
        Ok(s) => format!(
            r#"<pre class="text-xs whitespace-pre-wrap font-mono bg-base-200/50 rounded p-2">{}</pre>"#,
            html_escape(&s)
        ),
        Err(_) => String::new(),
    }
}

fn parts_visible_html(content: &Content, markdown: bool) -> String {
    let mut texts = Vec::new();
    for p in &content.parts {
        if let Some(t) = &p.text {
            if t == ZWSP || t.is_empty() {
                continue;
            }
            if markdown {
                texts.push(render_markdown(t));
            } else {
                texts.push(html_escape(t).replace('\n', "<br>"));
            }
        }
        if let Some(blob) = &p.inline_data {
            let name = if blob.display_name.is_empty() {
                "attachment"
            } else {
                blob.display_name.as_str()
            };
            texts.push(format!(
                r#"<div class="text-xs opacity-80 mt-1">[{}]</div>"#,
                html_escape(name)
            ));
        }
        if let Some(fc) = &p.function_call {
            texts.push(function_call_html(fc));
        }
    }
    if texts.is_empty() && content.role.eq_ignore_ascii_case(ROLE_MODEL) {
        texts.push("<em>…</em>".into());
    }
    texts.join("")
}
