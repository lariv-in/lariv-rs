use super::{
    handlers,
    keys::{MeetsHubTableKey, MeetsRoomChromeKey},
};

crate::define_plugin_routes! {
    plugin: MeetsTag;
    routes: [
        get HubRouteTag, "/meets", handlers::hub::hub, fragment(MeetsHubTableKey);
        get RoomCreateGetRouteTag, "/meets/create", handlers::hub::create_get, modal;
        post RoomCreatePostRouteTag, "/meets/create", handlers::hub::create_post;
        get RoomRouteTag, "/meets/{code}", handlers::rooms::room, fragment(MeetsRoomChromeKey);
        post RoomStartRouteTag, "/meets/{code}/start", bare handlers::rooms::start, redirect;
        post RoomLockRouteTag, "/meets/{code}/lock", handlers::rooms::lock, fragment(MeetsRoomChromeKey);
        post RoomLeaveRouteTag, "/meets/{code}/leave", bare handlers::rooms::leave, redirect;
        get JoinGetRouteTag, "/meets/{code}/join", handlers::join::join_get;
        post JoinPostRouteTag, "/meets/{code}/join", handlers::join::join_post;
        get SignalRouteTag, "/meets/{code}/signal", bare handlers::signal::upgrade, raw;
        get RecordingsRouteTag, "/meets/{code}/recordings", handlers::rooms::recordings;
    ]
}
