//! LLM Assistant HTTP routes — tagged entries on [`crate::http::HttpCapability`]'s route HList.

use crate::define_plugin_routes;

/// Assistant preferences. Empty allowlist: superuser only.
/// Deployments that show the assistant to another role leave this tag unchanged.
pub struct LlmPrefsAdmin;

/// Creating, editing, deleting, and importing skills. Superuser always may.
/// `admin` is the fallback. Deployments that show the assistant leave this tag unchanged.
pub struct LlmSkillsMutate;

use super::{
    handlers,
    keys::{
        CronJobDeleteModalKey, CronJobsTableKey, HistoryTableKey, SkillDeleteModalKey,
        SkillsTableKey,
    },
};

define_plugin_routes! {
    plugin: LlmAssistantTag;
    prefix: "/dashboard";
    routes: [
        get ChatIndexRouteTag, "/llm-assistant", handlers::chat::index;
        get ChatHistoryPanelRouteTag, "/llm-assistant/history-panel", bare handlers::chat::history_panel, raw;
        get ChatSidebarSessionRouteTag, "/llm-assistant/sidebar-chat/{id}", bare handlers::chat::sidebar_session, raw;
        get ChatWsRouteTag, "/llm-assistant/ws", bare handlers::ws::upgrade, raw;
        get PrefsGetRouteTag, "/llm-assistant/preferences", handlers::preferences::get, authorize(LlmPrefsAdmin, []);
        post PrefsPostRouteTag, "/llm-assistant/preferences", handlers::preferences::post, authorize(LlmPrefsAdmin, []);
        post ChatUploadRouteTag, "/llm-assistant/chat-upload", bare handlers::chat_upload::chat_upload, raw;
        get HistoryListRouteTag, "/llm-assistant/history", handlers::history::list, fragment(HistoryTableKey);
        get SkillsListRouteTag, "/llm-assistant/skills", handlers::skills::list, fragment(SkillsTableKey);
        get SkillsCreateGetRouteTag, "/llm-assistant/skills/create", handlers::skills::create_get, modal, authorize(LlmSkillsMutate, ["admin"]);
        post SkillsCreatePostRouteTag, "/llm-assistant/skills/create", handlers::skills::create_post, authorize(LlmSkillsMutate, ["admin"]);
        get SkillsDetailRouteTag, "/llm-assistant/skills/{id}", handlers::skills::detail;
        get SkillsUpdateGetRouteTag, "/llm-assistant/skills/{id}/update", handlers::skills::edit_get, modal, authorize(LlmSkillsMutate, ["admin"]);
        post SkillsUpdatePostRouteTag, "/llm-assistant/skills/{id}/update", handlers::skills::edit_post, authorize(LlmSkillsMutate, ["admin"]);
        get SkillsDeleteGetRouteTag, "/llm-assistant/skills/{id}/delete", handlers::skills::delete_get, modal, authorize(LlmSkillsMutate, ["admin"]);
        post SkillsDeletePostRouteTag, "/llm-assistant/skills/{id}/delete", bare handlers::skills::delete_post, fragment(SkillDeleteModalKey), authorize(LlmSkillsMutate, ["admin"]);
        get SkillsExportRouteTag, "/llm-assistant/skills/{id}/export", bare handlers::skills::export_skill_handler, file;
        get SkillsImportGetRouteTag, "/llm-assistant/skills/import", handlers::skills::import_get, authorize(LlmSkillsMutate, ["admin"]);
        post SkillsImportPostRouteTag, "/llm-assistant/skills/import", bare handlers::skills::import_post, redirect, authorize(LlmSkillsMutate, ["admin"]);
        get CronJobsListRouteTag, "/llm-assistant/cron-jobs", handlers::cron::list, fragment(CronJobsTableKey);
        get CronJobsCreateGetRouteTag, "/llm-assistant/cron-jobs/create", handlers::cron::create_get, modal;
        post CronJobsCreatePostRouteTag, "/llm-assistant/cron-jobs/create", handlers::cron::create_post;
        get CronJobsDetailRouteTag, "/llm-assistant/cron-jobs/{id}", handlers::cron::detail;
        post CronJobsRunPostRouteTag, "/llm-assistant/cron-jobs/{id}/run", bare handlers::cron::run_post, redirect;
        get CronJobsUpdateGetRouteTag, "/llm-assistant/cron-jobs/{id}/update", handlers::cron::edit_get, modal;
        post CronJobsUpdatePostRouteTag, "/llm-assistant/cron-jobs/{id}/update", handlers::cron::edit_post;
        get CronJobsDeleteGetRouteTag, "/llm-assistant/cron-jobs/{id}/delete", handlers::cron::delete_get, modal;
        post CronJobsDeletePostRouteTag, "/llm-assistant/cron-jobs/{id}/delete", bare handlers::cron::delete_post, fragment(CronJobDeleteModalKey);
    ]
}
