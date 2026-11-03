use super::{
    handlers,
    keys::{
        COMPANY_DELETE_MODAL, COMPANY_SELECT_MODAL, COMPANY_SELECT_TABLE, COMPANY_TABLE,
        CONTACT_DELETE_MODAL, CONTACT_SELECT_MODAL, CONTACT_SELECT_TABLE, CONTACT_TABLE,
    },
};

pub struct ContactsView;

pub struct ContactsMutate;

lariv_core::define_plugin_routes! {
    plugin: ContactsTag;
    prefix: "/dashboard";
    routes: [
        get ContactDefaultRouteTag, "/contacts", handlers::contacts::list, fragment(CONTACT_TABLE), authorize(ContactsView, []);
        get ContactCreateGetRouteTag, "/contacts/create", handlers::contacts::create_get, modal, authorize(ContactsMutate, []);
        post ContactCreatePostRouteTag, "/contacts/create", handlers::contacts::create_post, authorize(ContactsMutate, []);
        get ContactDetailRouteTag, "/contacts/{id}", handlers::contacts::detail, authorize(ContactsView, []);
        get ContactEditGetRouteTag, "/contacts/{id}/edit", handlers::contacts::edit_get, modal, authorize(ContactsMutate, []);
        post ContactEditPostRouteTag, "/contacts/{id}/edit", handlers::contacts::edit_post, authorize(ContactsMutate, []);
        get ContactDeleteGetRouteTag, "/contacts/{id}/delete", handlers::contacts::delete_get, modal, authorize(ContactsMutate, []);
        post ContactDeletePostRouteTag, "/contacts/{id}/delete", bare handlers::contacts::delete_post, fragment(CONTACT_DELETE_MODAL), authorize(ContactsMutate, []);
        get ContactFkSelectRouteTag, "/contacts/pick", handlers::contacts::select, fk_select(CONTACT_SELECT_TABLE, CONTACT_SELECT_MODAL), authorize(ContactsView, []);

        get CompanyDefaultRouteTag, "/companies", handlers::companies::list, fragment(COMPANY_TABLE), authorize(ContactsView, []);
        get CompanyCreateGetRouteTag, "/companies/create", handlers::companies::create_get, modal, authorize(ContactsMutate, []);
        post CompanyCreatePostRouteTag, "/companies/create", handlers::companies::create_post, authorize(ContactsMutate, []);
        get CompanyDetailRouteTag, "/companies/{id}", handlers::companies::detail, authorize(ContactsView, []);
        get CompanyEditGetRouteTag, "/companies/{id}/edit", handlers::companies::edit_get, modal, authorize(ContactsMutate, []);
        post CompanyEditPostRouteTag, "/companies/{id}/edit", handlers::companies::edit_post, authorize(ContactsMutate, []);
        get CompanyDeleteGetRouteTag, "/companies/{id}/delete", handlers::companies::delete_get, modal, authorize(ContactsMutate, []);
        post CompanyDeletePostRouteTag, "/companies/{id}/delete", bare handlers::companies::delete_post, fragment(COMPANY_DELETE_MODAL), authorize(ContactsMutate, []);
        get CompanyFkSelectRouteTag, "/companies/pick", handlers::companies::select, fk_select(COMPANY_SELECT_TABLE, COMPANY_SELECT_MODAL), authorize(ContactsView, []);
    ]
}
