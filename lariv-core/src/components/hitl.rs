//! Human-in-the-loop approval card for gated Rune functions.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::HtmlAttrs;

/// Pending HITL prompt: function name, JSON args, Run / Deny actions.
pub struct HitlApproval<'a> {
    pub request_id: &'a str,
    pub function_name: &'a str,
    pub args_json: &'a str,
    pub run_attrs: HtmlAttrs,
    pub deny_attrs: HtmlAttrs,
}

impl Default for HitlApproval<'_> {
    fn default() -> Self {
        Self {
            request_id: "",
            function_name: "",
            args_json: "{}",
            run_attrs: HtmlAttrs::new(),
            deny_attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a HITL approval card. Callers supply button attributes (e.g. `data-hitl-*`).
pub fn hitl_approval(opts: HitlApproval<'_>) -> Markup {
    let id = format!("llm_assistant_hitl_{}", opts.request_id);
    html! {
        div id=(id) class="rounded border border-warning/40 bg-warning/10 p-2 flex flex-col gap-2" {
            div class="text-sm font-medium" {
                "Approval required: "
                code { (opts.function_name) }
            }
            pre class="text-xs whitespace-pre-wrap font-mono text-base-content border border-base-content/15 rounded p-1 m-0" {
                (opts.args_json)
            }
            div class="flex gap-2" {
                (PreEscaped(format!(
                    r#"<button type="button" class="btn btn-primary btn-sm"{}>Run</button>"#,
                    opts.run_attrs.as_string()
                )))
                (PreEscaped(format!(
                    r#"<button type="button" class="btn btn-ghost btn-sm"{}>Deny</button>"#,
                    opts.deny_attrs.as_string()
                )))
            }
        }
    }
}

/// Replace a pending HITL card after the human decides.
pub fn hitl_resolved(request_id: &str, function_name: &str, approved: bool) -> Markup {
    let id = format!("llm_assistant_hitl_{}", request_id);
    let status = if approved { "ran" } else { "denied" };
    html! {
        div id=(id) class="rounded border border-base-content/15 p-2 text-sm" {
            code { (function_name) }
            " — "
            (status)
        }
    }
}
