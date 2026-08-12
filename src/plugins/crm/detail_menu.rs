//! Entity detail sidebars for CRM record pages.

use maud::{Markup, html};

use crate::components::{
    SidebarMenu, SidebarMenuItem, sidebar_menu, sidebar_menu_item_pane,
};

use super::routes::{
    CompanyDetailRouteTag, CompanyEditGetRouteTag, ContactDetailRouteTag,
    ContactEditGetRouteTag, ConvertedLeadDetailRouteTag, DealDetailRouteTag,
    DealEditGetRouteTag, FailedLeadDetailRouteTag, LeadDetailRouteTag,
    LeadEditGetRouteTag,
};

struct DetailMenuNavItem {
    title: &'static str,
    url: String,
    active: bool,
}

fn detail_sidebar_menu(menu_title: String, nav_items: &[DetailMenuNavItem]) -> Markup {
    sidebar_menu(SidebarMenu {
        title: menu_title.as_str(),
        children: {
            let mut children = Markup::default();
            for item in nav_items {
                children = html! {
                    (children)
                    (sidebar_menu_item_pane(SidebarMenuItem {
                        title: item.title,
                        url: &item.url,
                        active: item.active,
                        ..Default::default()
                    }))
                };
            }
            children
        },
    })
}

fn entity_detail_menu(
    menu_title: String,
    detail_url: String,
    edit_url: String,
    active: &str,
    can_edit: bool,
) -> Markup {
    let mut nav = vec![DetailMenuNavItem {
        title: "Detail",
        url: detail_url,
        active: active == "detail",
    }];
    if can_edit {
        nav.push(DetailMenuNavItem {
            title: "Edit",
            url: edit_url,
            active: active == "edit",
        });
    }
    detail_sidebar_menu(menu_title, &nav)
}

pub fn lead_detail_menu(display_name: &str, lead_id: i64, active: &str, can_edit: bool) -> Markup {
    entity_detail_menu(
        format!("Lead: {display_name}"),
        LeadDetailRouteTag::new(lead_id).url(),
        LeadEditGetRouteTag::new(lead_id).url(),
        active,
        can_edit,
    )
}

pub fn converted_lead_detail_menu(
    display_name: &str,
    converted_id: i64,
    lead_id: i64,
    active: &str,
    can_edit: bool,
) -> Markup {
    entity_detail_menu(
        format!("Converted lead: {display_name}"),
        ConvertedLeadDetailRouteTag::new(converted_id).url(),
        LeadEditGetRouteTag::new(lead_id).url(),
        active,
        can_edit,
    )
}

pub fn failed_lead_detail_menu(
    display_name: &str,
    failed_id: i64,
    lead_id: i64,
    active: &str,
    can_edit: bool,
) -> Markup {
    entity_detail_menu(
        format!("Failed lead: {display_name}"),
        FailedLeadDetailRouteTag::new(failed_id).url(),
        LeadEditGetRouteTag::new(lead_id).url(),
        active,
        can_edit,
    )
}

pub fn company_detail_menu(name: &str, id: i64, active: &str, can_edit: bool) -> Markup {
    entity_detail_menu(
        format!("Company: {name}"),
        CompanyDetailRouteTag::new(id).url(),
        CompanyEditGetRouteTag::new(id).url(),
        active,
        can_edit,
    )
}

pub fn contact_detail_menu(display_name: &str, id: i64, active: &str, can_edit: bool) -> Markup {
    entity_detail_menu(
        format!("Contact: {display_name}"),
        ContactDetailRouteTag::new(id).url(),
        ContactEditGetRouteTag::new(id).url(),
        active,
        can_edit,
    )
}

pub fn deal_detail_menu(name: &str, id: i64, active: &str, can_edit: bool) -> Markup {
    entity_detail_menu(
        format!("Deal: {name}"),
        DealDetailRouteTag::new(id).url(),
        DealEditGetRouteTag::new(id).url(),
        active,
        can_edit,
    )
}

pub fn lead_edit_menu(
    menu_title: String,
    detail_url: String,
    lead_id: i64,
    can_edit: bool,
) -> Markup {
    entity_detail_menu(
        menu_title,
        detail_url,
        LeadEditGetRouteTag::new(lead_id).url(),
        "edit",
        can_edit,
    )
}
