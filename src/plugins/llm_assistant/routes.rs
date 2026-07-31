//! LLM Assistant HTTP routes — tagged entries on [`HttpCapability`]'s route HList.

use crate::plugin_routes::define_plugin_routes;

use super::{
    LlmAssistantTag, handlers,
    templates::{
        ChatPage, ChatPageTag, ChatSessionPage, ChatSessionPageTag, ConfirmDeletePage,
        HistoryListPage, HistoryListPageTag, SkillConfirmDeletePageTag, SkillDetailPage,
        SkillDetailPageTag, SkillFormPage, SkillFormPageTag, SkillImportPage, SkillImportPageTag,
        SkillListPage, SkillListPageTag,
    },
};

define_plugin_routes! {
    plugin: LlmAssistantTag;
    proof: LlmAssistantRoutesProof;
    pages: [
        pane ChatIdx, ChatP => ChatPageTag, ChatPage;
        pane ChatSessionIdx, ChatSessionP => ChatSessionPageTag, ChatSessionPage;
        pane HistoryListIdx, HistoryListP => HistoryListPageTag, HistoryListPage;
        pane SkillListIdx, SkillListP => SkillListPageTag, SkillListPage;
        pane SkillFormIdx, SkillFormP => SkillFormPageTag, SkillFormPage;
        pane SkillDetailIdx, SkillDetailP => SkillDetailPageTag, SkillDetailPage;
        page ConfirmDeleteIdx, ConfirmDeleteP => SkillConfirmDeletePageTag, ConfirmDeletePage;
        page SkillImportIdx, SkillImportP => SkillImportPageTag, SkillImportPage;
    ];
    routes: [
        get ChatIndexRouteTag, "/llm-assistant", handlers::chat::index;
        post ChatNewSessionRouteTag, "/llm-assistant/new-session", bare handlers::chat::new_session;
        get ChatHistoryPanelRouteTag, "/llm-assistant/history-panel", bare handlers::chat::history_panel;
        get ChatSidebarSessionRouteTag, "/llm-assistant/sidebar-chat/{id}", bare handlers::chat::sidebar_session;
        get ChatWsRouteTag, "/llm-assistant/ws", bare handlers::ws::upgrade;
        get ChatSessionRouteTag, "/llm-assistant/c/{id}", handlers::chat::session;
        get HistoryListRouteTag, "/llm-assistant/history", handlers::history::list;
        get SkillsListRouteTag, "/llm-assistant/skills", handlers::skills::list;
        get SkillsCreateGetRouteTag, "/llm-assistant/skills/create", handlers::skills::create_get;
        post SkillsCreatePostRouteTag, "/llm-assistant/skills/create", handlers::skills::create_post;
        get SkillsDetailRouteTag, "/llm-assistant/skills/{id}", handlers::skills::detail;
        get SkillsUpdateGetRouteTag, "/llm-assistant/skills/{id}/update", handlers::skills::edit_get;
        post SkillsUpdatePostRouteTag, "/llm-assistant/skills/{id}/update", handlers::skills::edit_post;
        get SkillsDeleteGetRouteTag, "/llm-assistant/skills/{id}/delete", handlers::skills::delete_get;
        post SkillsDeletePostRouteTag, "/llm-assistant/skills/{id}/delete", bare handlers::skills::delete_post;
        get SkillsExportRouteTag, "/llm-assistant/skills/{id}/export", bare handlers::skills::export_skill_handler;
        get SkillsImportGetRouteTag, "/llm-assistant/skills/import", handlers::skills::import_get;
        post SkillsImportPostRouteTag, "/llm-assistant/skills/import", bare handlers::skills::import_post;
    ]
}
