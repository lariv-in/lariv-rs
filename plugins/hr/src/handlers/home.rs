//! `GET /dashboard/hr` — open the first HR section this role may use.

use axum::response::Redirect;

use crate::routes::{
    ApplicantHubRouteTag, AttendanceListRouteTag, AttendanceView, HolidayListRouteTag, HolidayView,
    HrPeopleView, LeaveApplicationsRouteTag, LeaveView,
};
use lariv_plugin_users::middleware::RequireAuth;
use lariv_plugin_users::role_authorization::{principal_allowed, roles_for};

pub async fn home(RequireAuth(ctx): RequireAuth) -> Redirect {
    if principal_allowed(&ctx, &roles_for::<HrPeopleView>()) {
        return Redirect::to(&ApplicantHubRouteTag.url());
    }
    if principal_allowed(&ctx, &roles_for::<HolidayView>()) {
        return Redirect::to(&HolidayListRouteTag.url());
    }
    if principal_allowed(&ctx, &roles_for::<AttendanceView>()) {
        return Redirect::to(&AttendanceListRouteTag.url());
    }
    if principal_allowed(&ctx, &roles_for::<LeaveView>()) {
        return Redirect::to(&LeaveApplicationsRouteTag.url());
    }
    Redirect::to(&HolidayListRouteTag.url())
}
