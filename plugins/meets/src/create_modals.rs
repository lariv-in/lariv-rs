use super::keys::MeetsCreateModalKey;
use super::routes::{RoomCreateGetRouteTag, RoomCreatePostRouteTag};

lariv_core::impl_create_modal!(
    MeetsCreateModalKey,
    RoomCreateGetRouteTag,
    RoomCreatePostRouteTag,
    "p_meets.RoomCreateForm"
);
