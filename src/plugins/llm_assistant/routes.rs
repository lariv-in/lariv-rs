//! LLM Assistant HTTP routes — tagged entries on [`crate::http::HttpCapability`]'s route HList.

use crate::define_plugin_routes;

use super::{
    handlers,
    keys::{HistoryTableKey, SkillDeleteModalKey, SkillsTableKey},
};

define_plugin_routes! {
    plugin: LlmAssistantTag;
    routes: [
        get ChatIndexRouteTag, "/llm-assistant", handlers::chat::index;
        post ChatNewSessionRouteTag, "/llm-assistant/new-session", bare handlers::chat::new_session, redirect;
        get ChatHistoryPanelRouteTag, "/llm-assistant/history-panel", bare handlers::chat::history_panel, raw;
        get ChatSidebarSessionRouteTag, "/llm-assistant/sidebar-chat/{id}", bare handlers::chat::sidebar_session, raw;
        get ChatWsRouteTag, "/llm-assistant/ws", bare handlers::ws::upgrade, raw;
        get ChatSessionRouteTag, "/llm-assistant/c/{id}", handlers::chat::session;
        get PrefsGetRouteTag, "/llm-assistant/preferences", handlers::preferences::get;
        post PrefsPostRouteTag, "/llm-assistant/preferences", handlers::preferences::post;
        get HistoryListRouteTag, "/llm-assistant/history", handlers::history::list, fragment(HistoryTableKey);
        get SkillsListRouteTag, "/llm-assistant/skills", handlers::skills::list, fragment(SkillsTableKey);
        get SkillsCreateGetRouteTag, "/llm-assistant/skills/create", handlers::skills::create_get, modal;
        post SkillsCreatePostRouteTag, "/llm-assistant/skills/create", handlers::skills::create_post;
        get SkillsDetailRouteTag, "/llm-assistant/skills/{id}", handlers::skills::detail;
        get SkillsUpdateGetRouteTag, "/llm-assistant/skills/{id}/update", handlers::skills::edit_get, modal;
        post SkillsUpdatePostRouteTag, "/llm-assistant/skills/{id}/update", handlers::skills::edit_post;
        get SkillsDeleteGetRouteTag, "/llm-assistant/skills/{id}/delete", handlers::skills::delete_get, modal;
        post SkillsDeletePostRouteTag, "/llm-assistant/skills/{id}/delete", bare handlers::skills::delete_post, fragment(SkillDeleteModalKey);
        get SkillsExportRouteTag, "/llm-assistant/skills/{id}/export", bare handlers::skills::export_skill_handler, file;
        get SkillsImportGetRouteTag, "/llm-assistant/skills/import", handlers::skills::import_get;
        post SkillsImportPostRouteTag, "/llm-assistant/skills/import", bare handlers::skills::import_post, redirect;
    ]
}
