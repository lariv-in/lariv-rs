//! Assistant app catalog tile.

use std::sync::Mutex;

use crate::apps::define_register_apps;

pub const LLM_ASSISTANT_APP_KEY: &str = "p_llm_assistant";

define_register_apps! {
    plugin: LlmAssistantTag;
    key: LLM_ASSISTANT_APP_KEY;
    name: "Assistant";
    href: crate::plugins::llm_assistant::routes::HistoryListRouteTag.url();
    icon: "sparkles";
    roles: ["superuser", "admin"];
}

/// Roles a deployment adds so the chat drawer matches an expanded app allowlist.
static SIDEBAR_EXTRA_ROLES: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Show the assistant drawer to `role` as well as superuser and `admin`.
pub fn allow_sidebar_role(role: impl Into<String>) {
    let role = role.into();
    let mut roles = SIDEBAR_EXTRA_ROLES.lock().expect("assistant sidebar roles");
    if !roles.iter().any(|existing| existing == &role) {
        roles.push(role);
    }
}

/// Whether the chat drawer should render for this principal.
pub fn sidebar_visible(role: Option<&str>, is_superuser: bool) -> bool {
    if is_superuser {
        return true;
    }
    let Some(role) = role else {
        return false;
    };
    if role == "admin" {
        return true;
    }
    SIDEBAR_EXTRA_ROLES
        .lock()
        .expect("assistant sidebar roles")
        .iter()
        .any(|allowed| allowed == role)
}
