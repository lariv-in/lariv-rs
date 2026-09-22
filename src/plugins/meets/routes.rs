use super::{
    handlers,
    keys::{MeetsDeleteModalKey, MeetsHubTableKey, MeetsRoomChromeKey},
};

crate::define_plugin_routes! {
    plugin: MeetsTag;
    routes: [
        get HubRouteTag, "/meets", handlers::hub::hub, fragment(MeetsHubTableKey);
        get RoomCreateGetRouteTag, "/meets/create", handlers::hub::create_get, modal;
        post RoomCreatePostRouteTag, "/meets/create", handlers::hub::create_post;
        get RoomRouteTag, "/meets/{code}", handlers::rooms::room, fragment(MeetsRoomChromeKey);
        get RoomLobbyRouteTag, "/meets/{code}/lobby", handlers::rooms::lobby;
        post RoomEnterRouteTag, "/meets/{code}/enter", bare handlers::rooms::enter, redirect;
        get RoomCallRouteTag, "/meets/{code}/call", handlers::rooms::call;
        post RoomStartRouteTag, "/meets/{code}/start", bare handlers::rooms::start, redirect;
        post RoomStopRouteTag, "/meets/{code}/stop", bare handlers::rooms::stop, redirect;
        post RoomLockRouteTag, "/meets/{code}/lock", handlers::rooms::lock, fragment(MeetsRoomChromeKey);
        post RoomLeaveRouteTag, "/meets/{code}/leave", bare handlers::rooms::leave, redirect;
        get JoinGetRouteTag, "/meets/{code}/join", handlers::join::join_get;
        post JoinPostRouteTag, "/meets/{code}/join", handlers::join::join_post;
        get RecordingsRouteTag, "/meets/{code}/recordings", handlers::rooms::recordings, modal;
        get RoomEditGetRouteTag, "/meets/{code}/edit", handlers::rooms::edit_get, modal;
        post RoomEditPostRouteTag, "/meets/{code}/edit", handlers::rooms::edit_post;
        get RoomDeleteGetRouteTag, "/meets/{code}/delete", handlers::rooms::delete_get, modal;
        post RoomDeletePostRouteTag, "/meets/{code}/delete", bare handlers::rooms::delete_post, fragment(MeetsDeleteModalKey);
    ]
}
