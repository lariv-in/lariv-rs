use super::{
    handlers,
    keys::{ContactDeleteModalKey, ContactSelectModalKey, ContactSelectTableKey, ContactTableKey},
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
    ]
}
