//! Assistant tester app catalog tile.

use lariv_core::apps::define_register_apps;

define_register_apps! {
    plugin: LlmAssistantTesterTag;
    key: "p_llm_assistant_tester";
    name: "Assistant Tester";
    href: crate::routes::TestCaseListRouteTag.url();
    icon: "beaker";
    roles: [lariv_plugin_users::roles::Admin];
}
