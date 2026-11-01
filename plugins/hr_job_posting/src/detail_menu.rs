//! Applicant detail sidebar.

use maud::Markup;

use lariv_core::components::{SidebarMenu, SidebarMenuItem, sidebar_menu, sidebar_menu_item_pane};

use crate::routes::ApplicantDetailRouteTag;

pub fn applicant_detail_menu(display_name: &str, applicant_id: i64, active: &str) -> Markup {
    let title = format!("Applicant: {display_name}");
    let url = ApplicantDetailRouteTag::new(applicant_id).url();
    sidebar_menu(SidebarMenu {
        title: title.as_str(),
        children: sidebar_menu_item_pane(SidebarMenuItem {
            title: "Detail",
            url: &url,
            active: active == "detail",
            ..Default::default()
        }),
    })
}
