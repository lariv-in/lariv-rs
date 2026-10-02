use super::{
    handlers,
    keys::{
        LeadDeleteModalKey, LeadHubTableKey, LeadTagDeleteModalKey, LeadTagLeadsTableKey,
        LeadTagSelectModalKey, LeadTagSelectTableKey, LeadTagTableKey, LeadUpdateDeleteModalKey,
        LeadUpdatesKey,
    },
};

pub struct CrmView;

pub struct CrmMutate;

crate::define_plugin_routes! {
    plugin: CrmTag;
    prefix: "/dashboard";
    routes: [
        get LeadDefaultRouteTag, "/crm/leads", handlers::leads::hub, fragment(LeadHubTableKey), authorize(CrmView, []);
        get LeadCreateGetRouteTag, "/crm/leads/create", handlers::leads::create_get, modal, authorize(CrmMutate, []);
        post LeadCreatePostRouteTag, "/crm/leads/create", handlers::leads::create_post, authorize(CrmMutate, []);
        get LeadDetailRouteTag, "/crm/leads/{id}", handlers::leads::detail, fragment(LeadUpdatesKey), authorize(CrmMutate, []);
        get LeadEditGetRouteTag, "/crm/leads/{id}/edit", handlers::leads::edit_get, modal, authorize(CrmMutate, []);
        post LeadEditPostRouteTag, "/crm/leads/{id}/edit", handlers::leads::edit_post, authorize(CrmMutate, []);
        get LeadDeleteGetRouteTag, "/crm/leads/{id}/delete", handlers::leads::delete_get, modal, authorize(CrmMutate, []);
        post LeadDeletePostRouteTag, "/crm/leads/{id}/delete", bare handlers::leads::delete_post, fragment(LeadDeleteModalKey), authorize(CrmMutate, []);
        get LeadConvertGetRouteTag, "/crm/leads/{id}/convert", handlers::leads::convert_get, modal, authorize(CrmView, []);
        post LeadConvertPostRouteTag, "/crm/leads/{id}/convert", handlers::leads::convert_post, authorize(CrmMutate, []);
        get LeadFailGetRouteTag, "/crm/leads/{id}/fail", handlers::leads::fail_get, modal, authorize(CrmView, []);
        post LeadFailPostRouteTag, "/crm/leads/{id}/fail", handlers::leads::fail_post, authorize(CrmMutate, []);
        get ConvertedLeadDetailRouteTag, "/crm/leads/converted/{id}", handlers::leads::converted_detail, fragment(LeadUpdatesKey), authorize(CrmMutate, []);
        post ConvertedLeadReactivatePostRouteTag, "/crm/leads/converted/{id}/reactivate", bare handlers::leads::converted_reactivate_post, redirect, authorize(CrmMutate, []);
        get FailedLeadDetailRouteTag, "/crm/leads/failed/{id}", handlers::leads::failed_detail, fragment(LeadUpdatesKey), authorize(CrmMutate, []);
        post FailedLeadReactivatePostRouteTag, "/crm/leads/failed/{id}/reactivate", bare handlers::leads::reactivate_post, redirect, authorize(CrmMutate, []);

        get LeadTagDefaultRouteTag, "/crm/lead-tags", handlers::lead_tags::list, fragment(LeadTagTableKey), authorize(CrmView, []);
        get LeadTagSelectRouteTag, "/crm/lead-tags/pick", handlers::lead_tags::select, multi_select(LeadTagSelectTableKey, LeadTagSelectModalKey), authorize(CrmView, []);
        get LeadTagCreateGetRouteTag, "/crm/lead-tags/create", handlers::lead_tags::create_get, modal, authorize(CrmMutate, []);
        post LeadTagCreatePostRouteTag, "/crm/lead-tags/create", handlers::lead_tags::create_post, authorize(CrmMutate, []);
        get LeadTagDetailRouteTag, "/crm/lead-tags/{id}", handlers::lead_tags::detail, fragment(LeadTagLeadsTableKey), authorize(CrmView, []);
        get LeadTagEditGetRouteTag, "/crm/lead-tags/{id}/edit", handlers::lead_tags::edit_get, modal, authorize(CrmMutate, []);
        post LeadTagEditPostRouteTag, "/crm/lead-tags/{id}/edit", handlers::lead_tags::edit_post, authorize(CrmMutate, []);
        get LeadTagDeleteGetRouteTag, "/crm/lead-tags/{id}/delete", handlers::lead_tags::delete_get, modal, authorize(CrmMutate, []);
        post LeadTagDeletePostRouteTag, "/crm/lead-tags/{id}/delete", bare handlers::lead_tags::delete_post, fragment(LeadTagDeleteModalKey), authorize(CrmMutate, []);

        post LeadUpdateAddPostRouteTag, "/crm/leads/{lead_id}/updates", handlers::lead_updates::add_post, param lead_id: i64, fragment(LeadUpdatesKey), authorize(CrmMutate, []);
        get LeadUpdateDetailRouteTag, "/crm/lead-updates/{id}", handlers::lead_updates::detail, authorize(CrmMutate, []);
        get LeadUpdateEditGetRouteTag, "/crm/lead-updates/{id}/edit", handlers::lead_updates::edit_get, modal, authorize(CrmMutate, []);
        post LeadUpdateEditPostRouteTag, "/crm/lead-updates/{id}/edit", handlers::lead_updates::edit_post, authorize(CrmMutate, []);
        get LeadUpdateDeleteGetRouteTag, "/crm/lead-updates/{id}/delete", handlers::lead_updates::delete_get, modal, authorize(CrmMutate, []);
        post LeadUpdateDeletePostRouteTag, "/crm/lead-updates/{id}/delete", bare handlers::lead_updates::delete_post, fragment(LeadUpdateDeleteModalKey), authorize(CrmMutate, []);
    ]
}
