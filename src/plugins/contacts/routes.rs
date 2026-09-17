use super::{
    handlers,
    keys::{
        CompanyDeleteModalKey, CompanySelectModalKey, CompanySelectTableKey, CompanyTableKey,
        ContactDeleteModalKey, ContactSelectModalKey, ContactSelectTableKey, ContactTableKey,
    },
};

crate::define_plugin_routes! {
    plugin: ContactsTag;
    routes: [
        get ContactDefaultRouteTag, "/contacts", handlers::contacts::list, fragment(ContactTableKey);
        get ContactCreateGetRouteTag, "/contacts/create", handlers::contacts::create_get, modal;
        post ContactCreatePostRouteTag, "/contacts/create", handlers::contacts::create_post;
        get ContactDetailRouteTag, "/contacts/{id}", handlers::contacts::detail;
        get ContactEditGetRouteTag, "/contacts/{id}/edit", handlers::contacts::edit_get, modal;
        post ContactEditPostRouteTag, "/contacts/{id}/edit", handlers::contacts::edit_post;
        get ContactDeleteGetRouteTag, "/contacts/{id}/delete", handlers::contacts::delete_get, modal;
        post ContactDeletePostRouteTag, "/contacts/{id}/delete", bare handlers::contacts::delete_post, fragment(ContactDeleteModalKey);
        get ContactFkSelectRouteTag, "/contacts/pick", handlers::contacts::select, fk_select(ContactSelectTableKey, ContactSelectModalKey);

        get CompanyDefaultRouteTag, "/companies", handlers::companies::list, fragment(CompanyTableKey);
        get CompanyCreateGetRouteTag, "/companies/create", handlers::companies::create_get, modal;
        post CompanyCreatePostRouteTag, "/companies/create", handlers::companies::create_post;
        get CompanyDetailRouteTag, "/companies/{id}", handlers::companies::detail;
        get CompanyEditGetRouteTag, "/companies/{id}/edit", handlers::companies::edit_get, modal;
        post CompanyEditPostRouteTag, "/companies/{id}/edit", handlers::companies::edit_post;
        get CompanyDeleteGetRouteTag, "/companies/{id}/delete", handlers::companies::delete_get, modal;
        post CompanyDeletePostRouteTag, "/companies/{id}/delete", bare handlers::companies::delete_post, fragment(CompanyDeleteModalKey);
        get CompanyFkSelectRouteTag, "/companies/pick", handlers::companies::select, fk_select(CompanySelectTableKey, CompanySelectModalKey);
    ]
}
