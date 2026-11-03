use super::{
    handlers,
    keys::{
        CUSTOMER_SELECT_MODAL, CUSTOMER_SELECT_TABLE, LEAD_DELETE_MODAL, LEAD_HUB_TABLE,
        LEAD_TAG_DELETE_MODAL, LEAD_TAG_LEADS_TABLE, LEAD_TAG_SELECT_MODAL, LEAD_TAG_SELECT_TABLE,
        LEAD_TAG_TABLE, LEAD_UPDATE_DELETE_MODAL, LEAD_UPDATES,
    },
};

pub struct CrmView;

pub struct CrmMutate;

lariv_core::define_plugin_routes! {
    plugin: CrmTag;
    prefix: "/dashboard";
    routes: [
        get LeadDefaultRouteTag, "/crm/leads", handlers::leads::hub, fragment(LEAD_HUB_TABLE), authorize(CrmView, []);
        get LeadCreateGetRouteTag, "/crm/leads/create", handlers::leads::create_get, modal, authorize(CrmMutate, []);
        post LeadCreatePostRouteTag, "/crm/leads/create", handlers::leads::create_post, authorize(CrmMutate, []);
        get LeadDetailRouteTag, "/crm/leads/{id}", handlers::leads::detail, fragment(LEAD_UPDATES), authorize(CrmMutate, []);
        get LeadEditGetRouteTag, "/crm/leads/{id}/edit", handlers::leads::edit_get, modal, authorize(CrmMutate, []);
        post LeadEditPostRouteTag, "/crm/leads/{id}/edit", handlers::leads::edit_post, authorize(CrmMutate, []);
        get LeadDeleteGetRouteTag, "/crm/leads/{id}/delete", handlers::leads::delete_get, modal, authorize(CrmMutate, []);
        post LeadDeletePostRouteTag, "/crm/leads/{id}/delete", bare handlers::leads::delete_post, fragment(LEAD_DELETE_MODAL), authorize(CrmMutate, []);
        get LeadConvertGetRouteTag, "/crm/leads/{id}/convert", handlers::leads::convert_get, modal, authorize(CrmView, []);
        post LeadConvertPostRouteTag, "/crm/leads/{id}/convert", handlers::leads::convert_post, authorize(CrmMutate, []);
        get LeadFailGetRouteTag, "/crm/leads/{id}/fail", handlers::leads::fail_get, modal, authorize(CrmView, []);
        post LeadFailPostRouteTag, "/crm/leads/{id}/fail", handlers::leads::fail_post, authorize(CrmMutate, []);
        get ConvertedLeadDetailRouteTag, "/crm/leads/converted/{id}", handlers::leads::converted_detail, fragment(LEAD_UPDATES), authorize(CrmMutate, []);
        post ConvertedLeadReactivatePostRouteTag, "/crm/leads/converted/{id}/reactivate", bare handlers::leads::converted_reactivate_post, redirect, authorize(CrmMutate, []);
        get FailedLeadDetailRouteTag, "/crm/leads/failed/{id}", handlers::leads::failed_detail, fragment(LEAD_UPDATES), authorize(CrmMutate, []);
        post FailedLeadReactivatePostRouteTag, "/crm/leads/failed/{id}/reactivate", bare handlers::leads::reactivate_post, redirect, authorize(CrmMutate, []);

        get CustomerFkSelectRouteTag, "/crm/customers/pick", handlers::customers::select, fk_select(CUSTOMER_SELECT_TABLE, CUSTOMER_SELECT_MODAL), authorize(CrmView, []);
        get CustomerCreateGetRouteTag, "/crm/customers/create", handlers::customers::create_get, modal, authorize(CrmMutate, []);
        post CustomerCreatePostRouteTag, "/crm/customers/create", handlers::customers::create_post, authorize(CrmMutate, []);

        get LeadTagDefaultRouteTag, "/crm/lead-tags", handlers::lead_tags::list, fragment(LEAD_TAG_TABLE), authorize(CrmView, []);
        get LeadTagSelectRouteTag, "/crm/lead-tags/pick", handlers::lead_tags::select, multi_select(LEAD_TAG_SELECT_TABLE, LEAD_TAG_SELECT_MODAL), authorize(CrmView, []);
        get LeadTagCreateGetRouteTag, "/crm/lead-tags/create", handlers::lead_tags::create_get, modal, authorize(CrmMutate, []);
        post LeadTagCreatePostRouteTag, "/crm/lead-tags/create", handlers::lead_tags::create_post, authorize(CrmMutate, []);
        get LeadTagDetailRouteTag, "/crm/lead-tags/{id}", handlers::lead_tags::detail, fragment(LEAD_TAG_LEADS_TABLE), authorize(CrmView, []);
        get LeadTagEditGetRouteTag, "/crm/lead-tags/{id}/edit", handlers::lead_tags::edit_get, modal, authorize(CrmMutate, []);
        post LeadTagEditPostRouteTag, "/crm/lead-tags/{id}/edit", handlers::lead_tags::edit_post, authorize(CrmMutate, []);
        get LeadTagDeleteGetRouteTag, "/crm/lead-tags/{id}/delete", handlers::lead_tags::delete_get, modal, authorize(CrmMutate, []);
        post LeadTagDeletePostRouteTag, "/crm/lead-tags/{id}/delete", bare handlers::lead_tags::delete_post, fragment(LEAD_TAG_DELETE_MODAL), authorize(CrmMutate, []);

        post LeadUpdateAddPostRouteTag, "/crm/leads/{lead_id}/updates", handlers::lead_updates::add_post, param lead_id: i64, fragment(LEAD_UPDATES), authorize(CrmMutate, []);
        get LeadUpdateDetailRouteTag, "/crm/lead-updates/{id}", handlers::lead_updates::detail, authorize(CrmMutate, []);
        get LeadUpdateEditGetRouteTag, "/crm/lead-updates/{id}/edit", handlers::lead_updates::edit_get, modal, authorize(CrmMutate, []);
        post LeadUpdateEditPostRouteTag, "/crm/lead-updates/{id}/edit", handlers::lead_updates::edit_post, authorize(CrmMutate, []);
        get LeadUpdateDeleteGetRouteTag, "/crm/lead-updates/{id}/delete", handlers::lead_updates::delete_get, modal, authorize(CrmMutate, []);
        post LeadUpdateDeletePostRouteTag, "/crm/lead-updates/{id}/delete", bare handlers::lead_updates::delete_post, fragment(LEAD_UPDATE_DELETE_MODAL), authorize(CrmMutate, []);
    ]
}
