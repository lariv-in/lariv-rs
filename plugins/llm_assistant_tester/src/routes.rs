//! Assistant tester HTTP routes.

use lariv_core::define_plugin_routes;

use super::{
    handlers,
    keys::{TEST_CASE_DELETE_MODAL, TEST_CASE_TABLE},
};

define_plugin_routes! {
    plugin: LlmAssistantTesterTag;
    prefix: "/dashboard";
    routes: [
        get TestCaseListRouteTag, "/llm-assistant-tester", handlers::list, fragment(TEST_CASE_TABLE);
        get TestCaseCreateGetRouteTag, "/llm-assistant-tester/create", handlers::create_get, modal;
        post TestCaseCreatePostRouteTag, "/llm-assistant-tester/create", handlers::create_post;
        get TestCaseDetailRouteTag, "/llm-assistant-tester/{id}", handlers::detail;
        get TestCaseEditGetRouteTag, "/llm-assistant-tester/{id}/edit", handlers::edit_get, modal;
        post TestCaseEditPostRouteTag, "/llm-assistant-tester/{id}/edit", handlers::edit_post;
        get TestCaseDeleteGetRouteTag, "/llm-assistant-tester/{id}/delete", handlers::delete_get, modal;
        post TestCaseDeletePostRouteTag, "/llm-assistant-tester/{id}/delete", bare handlers::delete_post, fragment(TEST_CASE_DELETE_MODAL);
    ]
}
