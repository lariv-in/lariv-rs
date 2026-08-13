use super::{
    handlers,
    keys::{
        CompanySelectModalKey, CompanySelectTableKey, CompanyTableKey, ContactSelectModalKey,
        ContactSelectTableKey, ContactTableKey, LeadHubTableKey, TaskTableKey,
    },
};

crate::define_plugin_routes! {
    plugin: CrmTag;
    routes: [
        get LeadDefaultRouteTag, "/crm/leads", handlers::leads::hub, fragment(LeadHubTableKey);
        get LeadCreateGetRouteTag, "/crm/leads/create", handlers::leads::create_get, modal;
        post LeadCreatePostRouteTag, "/crm/leads/create", handlers::leads::create_post;
        get LeadDetailRouteTag, "/crm/leads/{id}", handlers::leads::detail;
        get LeadEditGetRouteTag, "/crm/leads/{id}/edit", handlers::leads::edit_get, modal;
        post LeadEditPostRouteTag, "/crm/leads/{id}/edit", handlers::leads::edit_post;
        post LeadDeletePostRouteTag, "/crm/leads/{id}/delete", bare handlers::leads::delete_post, redirect;
        get LeadConvertGetRouteTag, "/crm/leads/{id}/convert", handlers::leads::convert_get, modal;
        post LeadConvertPostRouteTag, "/crm/leads/{id}/convert", handlers::leads::convert_post;
        get LeadFailGetRouteTag, "/crm/leads/{id}/fail", handlers::leads::fail_get, modal;
        post LeadFailPostRouteTag, "/crm/leads/{id}/fail", handlers::leads::fail_post;
        get ConvertedLeadDetailRouteTag, "/crm/leads/converted/{id}", handlers::leads::converted_detail;
        get FailedLeadDetailRouteTag, "/crm/leads/failed/{id}", handlers::leads::failed_detail;
        post FailedLeadReactivatePostRouteTag, "/crm/leads/failed/{id}/reactivate", bare handlers::leads::reactivate_post, redirect;

        get CompanyDefaultRouteTag, "/crm/companies", handlers::companies::list, fragment(CompanyTableKey);
        get CompanyCreateGetRouteTag, "/crm/companies/create", handlers::companies::create_get, modal;
        post CompanyCreatePostRouteTag, "/crm/companies/create", handlers::companies::create_post;
        get CompanyDetailRouteTag, "/crm/companies/{id}", handlers::companies::detail;
        get CompanyEditGetRouteTag, "/crm/companies/{id}/edit", handlers::companies::edit_get, modal;
        post CompanyEditPostRouteTag, "/crm/companies/{id}/edit", handlers::companies::edit_post;
        post CompanyDeletePostRouteTag, "/crm/companies/{id}/delete", bare handlers::companies::delete_post, redirect;
        get CompanyFkSelectRouteTag, "/crm/companies/pick", handlers::companies::select, fk_select(CompanySelectTableKey, CompanySelectModalKey);

        get ContactDefaultRouteTag, "/crm/contacts", handlers::contacts::list, fragment(ContactTableKey);
        get ContactCreateGetRouteTag, "/crm/contacts/create", handlers::contacts::create_get, modal;
        post ContactCreatePostRouteTag, "/crm/contacts/create", handlers::contacts::create_post;
        get ContactDetailRouteTag, "/crm/contacts/{id}", handlers::contacts::detail;
        get ContactEditGetRouteTag, "/crm/contacts/{id}/edit", handlers::contacts::edit_get, modal;
        post ContactEditPostRouteTag, "/crm/contacts/{id}/edit", handlers::contacts::edit_post;
        post ContactDeletePostRouteTag, "/crm/contacts/{id}/delete", bare handlers::contacts::delete_post, redirect;
        get ContactFkSelectRouteTag, "/crm/contacts/pick", handlers::contacts::select, fk_select(ContactSelectTableKey, ContactSelectModalKey);

        get TaskDefaultRouteTag, "/crm/tasks", handlers::tasks::hub, fragment(TaskTableKey);
        get TaskCreateGetRouteTag, "/crm/tasks/create", handlers::tasks::create_get, modal;
        post TaskCreatePostRouteTag, "/crm/tasks/create", handlers::tasks::create_post;
        get TaskDetailRouteTag, "/crm/tasks/{id}", handlers::tasks::detail;
        get CompletedTaskDetailRouteTag, "/crm/tasks/completed/{id}", handlers::tasks::completed_detail;
        get TaskEditGetRouteTag, "/crm/tasks/{id}/edit", handlers::tasks::edit_get, modal;
        post TaskEditPostRouteTag, "/crm/tasks/{id}/edit", handlers::tasks::edit_post;
        post TaskCompletePostRouteTag, "/crm/tasks/{id}/complete", bare handlers::tasks::complete_post, redirect;
        post TaskDeletePostRouteTag, "/crm/tasks/{id}/delete", bare handlers::tasks::delete_post, redirect;

        get LeadUpdateCreateGetRouteTag, "/crm/leads/{lead_id}/updates/create", handlers::lead_updates::create_get, param lead_id: i64, modal;
        post LeadUpdateCreatePostRouteTag, "/crm/leads/{lead_id}/updates/create", handlers::lead_updates::create_post, param lead_id: i64;
        get LeadUpdateDetailRouteTag, "/crm/lead-updates/{id}", handlers::lead_updates::detail;
        get LeadUpdateEditGetRouteTag, "/crm/lead-updates/{id}/edit", handlers::lead_updates::edit_get, modal;
        post LeadUpdateEditPostRouteTag, "/crm/lead-updates/{id}/edit", handlers::lead_updates::edit_post;
        post LeadUpdateDeletePostRouteTag, "/crm/lead-updates/{id}/delete", bare handlers::lead_updates::delete_post, redirect;
    ]
}
