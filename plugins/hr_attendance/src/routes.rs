use lariv_plugin_users::roles::Unassigned;

use super::{
    handlers,
    keys::{AttendanceDeleteModalKey, AttendanceTableKey},
};

pub struct AttendanceView;

pub struct AttendanceMutate;

lariv_core::define_plugin_routes! {
    plugin: HrAttendanceTag;
    prefix: "/dashboard";
    routes: [
        get AttendanceListRouteTag, "/hr/attendances", handlers::attendances::list, fragment(AttendanceTableKey), authorize(AttendanceView, [Unassigned]);
        get AttendanceCreateGetRouteTag, "/hr/attendances/create", handlers::attendances::create_get, modal, authorize(AttendanceMutate, []);
        post AttendanceCreatePostRouteTag, "/hr/attendances/create", handlers::attendances::create_post, authorize(AttendanceMutate, []);
        post AttendancePunchInRouteTag, "/hr/attendances/punch-in", handlers::attendances::punch_in_post, fragment(AttendanceTableKey), authorize(AttendanceView, [Unassigned]);
        post AttendancePunchOutRouteTag, "/hr/attendances/punch-out", handlers::attendances::punch_out_post, fragment(AttendanceTableKey), authorize(AttendanceView, [Unassigned]);
        get AttendanceDetailRouteTag, "/hr/attendances/{id}", handlers::attendances::detail, authorize(AttendanceView, [Unassigned]);
        get AttendanceEditGetRouteTag, "/hr/attendances/{id}/edit", handlers::attendances::edit_get, modal, authorize(AttendanceMutate, []);
        post AttendanceEditPostRouteTag, "/hr/attendances/{id}/edit", handlers::attendances::edit_post, authorize(AttendanceMutate, []);
        get AttendanceDeleteGetRouteTag, "/hr/attendances/{id}/delete", handlers::attendances::delete_get, modal, authorize(AttendanceMutate, []);
        post AttendanceDeletePostRouteTag, "/hr/attendances/{id}/delete", bare handlers::attendances::delete_post, fragment(AttendanceDeleteModalKey), authorize(AttendanceMutate, []);
    ]
}
