use super::{
    handlers,
    keys::{
        FORM_DELETE_MODAL, FORM_DETAIL_RESPONSES_TABLE, FORM_RESPONSE_DELETE_MODAL,
        FORM_RESPONSE_SELECT_MODAL, FORM_RESPONSE_SELECT_TABLE, FORM_SELECT_MODAL,
        FORM_SELECT_TABLE, FORM_TABLE,
    },
};

lariv_core::define_plugin_routes! {
    plugin: FormsTag;
    prefix: "/dashboard";
    routes: [
        get FormListRouteTag, "/forms", handlers::forms::list, fragment(FORM_TABLE);
        get FormCreateGetRouteTag, "/forms/create", handlers::forms::create_get, modal;
        post FormCreatePostRouteTag, "/forms/create", handlers::forms::create_post;
        get FormDetailRouteTag, "/forms/{id}", handlers::forms::detail, fragment(FORM_DETAIL_RESPONSES_TABLE);
        get FormEditGetRouteTag, "/forms/{id}/edit", handlers::forms::edit_get, modal;
        post FormEditPostRouteTag, "/forms/{id}/edit", handlers::forms::edit_post;
        get FormDeleteGetRouteTag, "/forms/{id}/delete", handlers::forms::delete_get, modal;
        post FormDeletePostRouteTag, "/forms/{id}/delete", bare handlers::forms::delete_post, fragment(FORM_DELETE_MODAL);
        get FormFkSelectRouteTag, "/forms/pick", handlers::forms::select, fk_select(FORM_SELECT_TABLE, FORM_SELECT_MODAL);
        get FormResponseFkSelectRouteTag, "/forms/responses/pick", handlers::responses::select, fk_select(FORM_RESPONSE_SELECT_TABLE, FORM_RESPONSE_SELECT_MODAL);

        get FormResponseCreateGetRouteTag, "/forms/responses/create", handlers::responses::create_get, modal;
        post FormResponseCreatePostRouteTag, "/forms/responses/create", handlers::responses::create_post;
        get FormResponseDetailRouteTag, "/forms/responses/{id}", handlers::responses::detail;
        get FormResponseEditGetRouteTag, "/forms/responses/{id}/edit", handlers::responses::edit_get, modal;
        post FormResponseEditPostRouteTag, "/forms/responses/{id}/edit", handlers::responses::edit_post;
        get FormResponseDeleteGetRouteTag, "/forms/responses/{id}/delete", handlers::responses::delete_get, modal;
        post FormResponseDeletePostRouteTag, "/forms/responses/{id}/delete", bare handlers::responses::delete_post, fragment(FORM_RESPONSE_DELETE_MODAL);

        get FormPublicGetRouteTag, "/f/{uid}", root bare handlers::public::get, raw;
        post FormPublicPostRouteTag, "/f/{uid}", root bare handlers::public::post, raw;
        get FormPublicBackgroundRouteTag, "/f/{uid}/background", root bare handlers::public::background, file;
    ]
}
