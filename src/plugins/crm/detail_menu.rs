//! Entity detail sidebars for CRM record pages.

use maud::{Markup, html};

use crate::components::{SidebarMenu, SidebarMenuItem, sidebar_menu, sidebar_menu_item_pane};

use super::routes::{
    CompanyDetailRouteTag, CompletedTaskDetailRouteTag, ContactDetailRouteTag,
    ConvertedLeadDetailRouteTag, FailedLeadDetailRouteTag, LeadDetailRouteTag,
    LeadTagDetailRouteTag, LeadTimelineRouteTag, TaskDetailRouteTag,
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

fn entity_detail_menu(menu_title: String, detail_url: String, active: &str) -> Markup {
    detail_sidebar_menu(
        menu_title,
        &[DetailMenuNavItem {
            title: "Detail",
            url: detail_url,
            active: active == "detail",
        }],
    )
}

pub fn lead_detail_menu(display_name: &str, lead_id: i64, active: &str) -> Markup {
    detail_sidebar_menu(
        format!("Lead: {display_name}"),
        &[
            DetailMenuNavItem {
                title: "Detail",
                url: LeadDetailRouteTag::new(lead_id).url(),
                active: active == "detail",
            },
            DetailMenuNavItem {
                title: "Timeline",
                url: LeadTimelineRouteTag::new(lead_id).url(),
                active: active == "timeline",
            },
        ],
    )
}

pub fn converted_lead_detail_menu(
    display_name: &str,
    converted_id: i64,
    lead_id: i64,
    active: &str,
) -> Markup {
    detail_sidebar_menu(
        format!("Converted lead: {display_name}"),
        &[
            DetailMenuNavItem {
                title: "Detail",
                url: ConvertedLeadDetailRouteTag::new(converted_id).url(),
                active: active == "detail",
            },
            DetailMenuNavItem {
                title: "Timeline",
                url: LeadTimelineRouteTag::new(lead_id).url(),
                active: active == "timeline",
            },
        ],
    )
}

pub fn failed_lead_detail_menu(
    display_name: &str,
    failed_id: i64,
    lead_id: i64,
    active: &str,
) -> Markup {
    detail_sidebar_menu(
        format!("Failed lead: {display_name}"),
        &[
            DetailMenuNavItem {
                title: "Detail",
                url: FailedLeadDetailRouteTag::new(failed_id).url(),
                active: active == "detail",
            },
            DetailMenuNavItem {
                title: "Timeline",
                url: LeadTimelineRouteTag::new(lead_id).url(),
                active: active == "timeline",
            },
        ],
    )
}

pub fn lead_tag_detail_menu(name: &str, id: i64, active: &str) -> Markup {
    entity_detail_menu(
        format!("Tag: {name}"),
        LeadTagDetailRouteTag::new(id).url(),
        active,
    )
}

pub fn company_detail_menu(name: &str, id: i64, active: &str) -> Markup {
    entity_detail_menu(
        format!("Company: {name}"),
        CompanyDetailRouteTag::new(id).url(),
        active,
    )
}

pub fn contact_detail_menu(display_name: &str, id: i64, active: &str) -> Markup {
    entity_detail_menu(
        format!("Contact: {display_name}"),
        ContactDetailRouteTag::new(id).url(),
        active,
    )
}

pub fn task_detail_menu(title: &str, id: i64, active: &str) -> Markup {
    entity_detail_menu(
        format!("Task: {title}"),
        TaskDetailRouteTag::new(id).url(),
        active,
    )
}

pub fn completed_task_detail_menu(title: &str, id: i64, active: &str) -> Markup {
    entity_detail_menu(
        format!("Completed task: {title}"),
        CompletedTaskDetailRouteTag::new(id).url(),
        active,
    )
}
