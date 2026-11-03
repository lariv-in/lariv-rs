use lariv_core::web::CreateModal;

use super::keys::MEETS_CREATE_MODAL;
use super::routes::{RoomCreateGetRouteTag, RoomCreatePostRouteTag};

pub const MEETS_CREATE: CreateModal<RoomCreateGetRouteTag, RoomCreatePostRouteTag> = CreateModal {
    key: MEETS_CREATE_MODAL,
    form_name: "p_meets.RoomCreateForm",
    get: RoomCreateGetRouteTag,
    post: RoomCreatePostRouteTag,
};
