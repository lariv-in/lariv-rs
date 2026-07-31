//! Assistant app catalog tile (Go `p_llm_assistant` `PluginTypeApp` registration).

use crate::apps::define_register_apps;


define_register_apps! {
    plugin: LlmAssistantTag;
    key: "p_llm_assistant";
    name: "Assistant";
    href: "/llm-assistant";
    icon: "sparkles";
    roles: ["superuser", "admin"];
}
