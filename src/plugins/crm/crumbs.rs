//! Breadcrumb trails for CRM pages.

use maud::Markup;

use crate::components::{Crumb, breadcrumbs};

use super::routes::{
    CompanyDefaultRouteTag, CompanyDetailRouteTag, CompletedTaskDetailRouteTag,
    ContactDefaultRouteTag, ContactDetailRouteTag, ConvertedLeadDetailRouteTag,
    FailedLeadDetailRouteTag, LeadDefaultRouteTag, LeadDetailRouteTag, LeadTagDefaultRouteTag,
    LeadTagDetailRouteTag, TaskDefaultRouteTag, TaskDetailRouteTag,
};

fn leads_tab_url(tab: &str) -> String {
    crate::http::RouteQueryBuilder::new(LeadDefaultRouteTag)
        .query("tab", tab)
        .build()
}

fn entity_crumbs(
    list_label: &'static str,
    list_url: &str,
    name: &str,
    detail_url: &str,
    action: Option<&str>,
) -> Markup {
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: list_label,
                href: Some(list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: list_label,
                href: Some(list_url),
            },
            Crumb {
                label: name,
                href: Some(detail_url),
            },
            Crumb {
                label: act,
                href: None,
            },
        ]),
    }
}

pub fn leads_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Leads",
        href: None,
    }])
}

pub fn lead_crumbs(name: &str, lead_id: i64, action: Option<&str>) -> Markup {
    entity_crumbs(
        "Leads",
        &leads_tab_url("active"),
        name,
        &LeadDetailRouteTag::new(lead_id).url(),
        action,
    )
}

pub fn lead_update_crumbs(name: &str, lead_id: i64, update_label: &str) -> Markup {
    breadcrumbs(&[
        Crumb {
            label: "Leads",
            href: Some(&leads_tab_url("active")),
        },
        Crumb {
            label: name,
            href: Some(&LeadDetailRouteTag::new(lead_id).url()),
        },
        Crumb {
            label: update_label,
            href: None,
        },
    ])
}

pub fn converted_lead_crumbs(name: &str, converted_id: i64, action: Option<&str>) -> Markup {
    entity_crumbs(
        "Leads",
        &leads_tab_url("converted"),
        name,
        &ConvertedLeadDetailRouteTag::new(converted_id).url(),
        action,
    )
}

pub fn failed_lead_crumbs(name: &str, failed_id: i64, action: Option<&str>) -> Markup {
    entity_crumbs(
        "Leads",
        &leads_tab_url("failed"),
        name,
        &FailedLeadDetailRouteTag::new(failed_id).url(),
        action,
    )
}

pub fn lead_tags_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Tags",
        href: None,
    }])
}

pub fn lead_tag_crumbs(name: &str, id: i64, action: Option<&str>) -> Markup {
    let list_url = LeadTagDefaultRouteTag.url();
    entity_crumbs(
        "Tags",
        &list_url,
        name,
        &LeadTagDetailRouteTag::new(id).url(),
        action,
    )
}

pub fn companies_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Companies",
        href: None,
    }])
}

pub fn company_crumbs(name: &str, id: i64, action: Option<&str>) -> Markup {
    let list_url = CompanyDefaultRouteTag.url();
    entity_crumbs(
        "Companies",
        &list_url,
        name,
        &CompanyDetailRouteTag::new(id).url(),
        action,
    )
}

pub fn contacts_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Contacts",
        href: None,
    }])
}

pub fn contact_crumbs(name: &str, id: i64, action: Option<&str>) -> Markup {
    let list_url = ContactDefaultRouteTag.url();
    entity_crumbs(
        "Contacts",
        &list_url,
        name,
        &ContactDetailRouteTag::new(id).url(),
        action,
    )
}

fn tasks_tab_url(tab: &str) -> String {
    crate::http::RouteQueryBuilder::new(TaskDefaultRouteTag)
        .query("tab", tab)
        .build()
}

pub fn tasks_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Tasks",
        href: None,
    }])
}

pub fn task_crumbs(name: &str, id: i64, action: Option<&str>) -> Markup {
    entity_crumbs(
        "Tasks",
        &tasks_tab_url("uncompleted"),
        name,
        &TaskDetailRouteTag::new(id).url(),
        action,
    )
}

pub fn completed_task_crumbs(name: &str, id: i64, action: Option<&str>) -> Markup {
    entity_crumbs(
        "Tasks",
        &tasks_tab_url("completed"),
        name,
        &CompletedTaskDetailRouteTag::new(id).url(),
        action,
    )
}
