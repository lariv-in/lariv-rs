//! Holidays link patched onto the shared HR app sidebar.

use lariv_plugin_hr::hr_sidebar::{HrSidebarRegistrar, HrSidebarRegistry, link_with_prefixes};
use lariv_plugin_users::role_authorization::roles_for;

use crate::routes::{HolidayListRouteTag, HolidayView};

const HOLIDAYS_MATCH_PREFIXES: &[&str] = &[HolidayListRouteTag::PATH];

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl HrSidebarRegistrar for Hook {
    fn register_hr_sidebar(self, cap: HrSidebarRegistry) -> HrSidebarRegistry {
        cap.push(
            link_with_prefixes::<HolidayListRouteTag>(
                "holidays",
                "Holidays",
                30,
                None,
                HOLIDAYS_MATCH_PREFIXES,
            )
            .with_roles(roles_for::<HolidayView>),
        )
    }
}
