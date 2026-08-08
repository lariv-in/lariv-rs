//! Assistant app catalog tile.

use crate::apps::define_register_apps;


define_register_apps! {
    plugin: LlmAssistantTag;
    key: "p_llm_assistant";
    name: "Assistant";
    href: "/llm-assistant/history";
    icon: "sparkles";
    roles: ["superuser", "admin"];
}
