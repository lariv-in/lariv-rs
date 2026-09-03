//! Tooltip copy for LLM assistant skill form fields.

use crate::plugins::llm_assistant::hitl::HitlCapability;
use crate::rune_env::RuneEnvCapability;

/// Base hint for the skill Content field (Rune overview). Binding catalogs are patched
/// from [`RuneEnvCapability::binding_docs`] for whatever plugins are mounted.
pub const CONTENT_INTRO: &str = "\
Skill content is instruction text for the assistant. When a skill needs computation or \
side effects, the assistant evaluates Rune scripts (https://github.com/rune-rs/rune) via \
the `run_rune` / `run_rune_file` tools — not by treating this field as source code.

Rune is a sandboxed scripting language used for arithmetic, data transforms, and calls to \
registered lariv bindings. Prefer a `pub fn main()` entrypoint. Object literals use \
`#{ ... }` (not `{ ... }`). Call `list_rune_env` before writing scripts to confirm live \
binding names. If the schema of a Rune function is unknown, call `get_rune_env` with its \
name — do not guess arguments or return types.

Kinds of names available in the Rune environment:
• Deployment / plugin bindings — request-scoped helpers registered by mounted plugins \
(listed below for this deployment).
• Static values — constants plugins register once (rarely used in skills).
• HITL bindings — same call style, but the assistant pauses until a human clicks Run \
in chat (email / unattended turns cannot execute them).

Document which bindings a skill needs, when to call them, and how to read their return \
values. Most bindings take one object argument and return a JSON-shaped object or a \
simple scalar.";

/// Compose the Content field hint: intro plus signatures from mounted Rune env and HITL plugins.
pub fn content_hint(rune_env: &RuneEnvCapability, hitl: &HitlCapability) -> String {
    let docs = rune_env.binding_docs();
    let hitl_docs = hitl.binding_docs();
    if docs.is_empty() && hitl_docs.is_empty() {
        format!("{CONTENT_INTRO}\n\nNo plugin bindings are registered in this deployment.")
    } else {
        let mut out = format!("{CONTENT_INTRO}\n\nRegistered bindings in this deployment:\n");
        for doc in docs {
            out.push('•');
            out.push(' ');
            out.push_str(doc);
            out.push('\n');
        }
        if !hitl_docs.is_empty() {
            out.push_str("HITL bindings (require human approval):\n");
            for doc in hitl_docs {
                out.push('•');
                out.push(' ');
                out.push_str(doc);
                out.push('\n');
            }
        }
        out
    }
}

#[cfg(all(test, feature = "cap-llm"))]
mod tests {
    use super::*;
    use crate::rune_env::NativeBinding;
    use std::sync::Arc;

    #[test]
    fn content_hint_lists_only_registered_docs() {
        let mut env = RuneEnvCapability::new();
        env.register_contextual(
            "search_products",
            "search_products(#{ query: string }) -> #{ results: [...] }",
            |_ctx| NativeBinding::Function(Arc::new(|_ctx, _args| Err("unused".into()))),
        );
        let hint = content_hint(
            &env,
            &crate::plugins::llm_assistant::hitl::HitlCapability::new(),
        );
        assert!(hint.contains(CONTENT_INTRO));
        assert!(hint.contains("search_products(#{ query: string })"));
        assert!(!hint.contains("create_invoice"));
        assert!(!hint.contains("No plugin bindings"));
    }

    #[test]
    fn content_hint_empty_registry() {
        let hint = content_hint(
            &RuneEnvCapability::new(),
            &crate::plugins::llm_assistant::hitl::HitlCapability::new(),
        );
        assert!(hint.contains("No plugin bindings are registered"));
    }

    #[test]
    fn content_hint_lists_hitl_docs() {
        let mut hitl = crate::plugins::llm_assistant::hitl::HitlCapability::new();
        hitl.register(
            "delete_draft_invoice",
            "delete_draft_invoice(#{ id: int }) -> ()  // requires human approval",
            |_ctx| NativeBinding::Function(Arc::new(|_ctx, _args| Err("unused".into()))),
        );
        let hint = content_hint(&RuneEnvCapability::new(), &hitl);
        assert!(hint.contains("HITL bindings (require human approval)"));
        assert!(hint.contains("delete_draft_invoice(#{ id: int })"));
    }
}
