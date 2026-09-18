use super::{
    handlers,
    keys::{
        FormDeleteModalKey, FormDetailResponsesTableKey, FormResponseDeleteModalKey,
        FormSelectModalKey, FormSelectTableKey, FormTableKey,
    },
};

crate::define_plugin_routes! {
    plugin: FormsTag;
    routes: [
        get FormListRouteTag, "/forms", handlers::forms::list, fragment(FormTableKey);
        get FormCreateGetRouteTag, "/forms/create", handlers::forms::create_get, modal;
        post FormCreatePostRouteTag, "/forms/create", handlers::forms::create_post;
        get FormDetailRouteTag, "/forms/{id}", handlers::forms::detail, fragment(FormDetailResponsesTableKey);
        get FormEditGetRouteTag, "/forms/{id}/edit", handlers::forms::edit_get, modal;
        post FormEditPostRouteTag, "/forms/{id}/edit", handlers::forms::edit_post;
        get FormDeleteGetRouteTag, "/forms/{id}/delete", handlers::forms::delete_get, modal;
        post FormDeletePostRouteTag, "/forms/{id}/delete", bare handlers::forms::delete_post, fragment(FormDeleteModalKey);
        get FormFkSelectRouteTag, "/forms/pick", handlers::forms::select, fk_select(FormSelectTableKey, FormSelectModalKey);

        get FormResponseCreateGetRouteTag, "/forms/responses/create", handlers::responses::create_get, modal;
        post FormResponseCreatePostRouteTag, "/forms/responses/create", handlers::responses::create_post;
        get FormResponseDetailRouteTag, "/forms/responses/{id}", handlers::responses::detail;
        get FormResponseEditGetRouteTag, "/forms/responses/{id}/edit", handlers::responses::edit_get, modal;
        post FormResponseEditPostRouteTag, "/forms/responses/{id}/edit", handlers::responses::edit_post;
        get FormResponseDeleteGetRouteTag, "/forms/responses/{id}/delete", handlers::responses::delete_get, modal;
        post FormResponseDeletePostRouteTag, "/forms/responses/{id}/delete", bare handlers::responses::delete_post, fragment(FormResponseDeleteModalKey);
    ]
}
