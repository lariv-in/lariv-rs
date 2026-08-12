use super::{
    handlers,
    keys::{
        CompanySelectModalKey, CompanySelectTableKey, CompanyTableKey,
        ContactSelectModalKey, ContactSelectTableKey, ContactTableKey,
        DealTableKey, LeadHubTableKey,
    },
};

crate::define_plugin_routes! {
    plugin: CrmTag;
    routes: [
        get LeadDefaultRouteTag, "/crm/leads", handlers::leads::hub, fragment(LeadHubTableKey);
        get LeadCreateGetRouteTag, "/crm/leads/create", handlers::leads::create_get, modal;
        post LeadCreatePostRouteTag, "/crm/leads/create", handlers::leads::create_post;
        get LeadDetailRouteTag, "/crm/leads/{id}", handlers::leads::detail;
        get LeadEditGetRouteTag, "/crm/leads/{id}/edit", handlers::leads::edit_get;
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
        get CompanyEditGetRouteTag, "/crm/companies/{id}/edit", handlers::companies::edit_get;
        post CompanyEditPostRouteTag, "/crm/companies/{id}/edit", handlers::companies::edit_post;
        post CompanyDeletePostRouteTag, "/crm/companies/{id}/delete", bare handlers::companies::delete_post, redirect;
        get CompanyFkSelectRouteTag, "/crm/companies/pick", handlers::companies::select, fk_select(CompanySelectTableKey, CompanySelectModalKey);

        get ContactDefaultRouteTag, "/crm/contacts", handlers::contacts::list, fragment(ContactTableKey);
        get ContactCreateGetRouteTag, "/crm/contacts/create", handlers::contacts::create_get, modal;
        post ContactCreatePostRouteTag, "/crm/contacts/create", handlers::contacts::create_post;
        get ContactDetailRouteTag, "/crm/contacts/{id}", handlers::contacts::detail;
        get ContactEditGetRouteTag, "/crm/contacts/{id}/edit", handlers::contacts::edit_get;
        post ContactEditPostRouteTag, "/crm/contacts/{id}/edit", handlers::contacts::edit_post;
        post ContactDeletePostRouteTag, "/crm/contacts/{id}/delete", bare handlers::contacts::delete_post, redirect;
        get ContactFkSelectRouteTag, "/crm/contacts/pick", handlers::contacts::select, fk_select(ContactSelectTableKey, ContactSelectModalKey);

        get DealDefaultRouteTag, "/crm/deals", handlers::deals::list, fragment(DealTableKey);
        get DealCreateGetRouteTag, "/crm/deals/create", handlers::deals::create_get, modal;
        post DealCreatePostRouteTag, "/crm/deals/create", handlers::deals::create_post;
        get DealDetailRouteTag, "/crm/deals/{id}", handlers::deals::detail;
        get DealEditGetRouteTag, "/crm/deals/{id}/edit", handlers::deals::edit_get;
        post DealEditPostRouteTag, "/crm/deals/{id}/edit", handlers::deals::edit_post;
        post DealDeletePostRouteTag, "/crm/deals/{id}/delete", bare handlers::deals::delete_post, redirect;
    ]
}
