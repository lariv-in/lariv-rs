//! Breadcrumb trails for CRM pages.

use maud::Markup;

use crate::components::{Crumb, breadcrumbs};

use super::routes::{
    CompanyDefaultRouteTag, CompanyDetailRouteTag, ContactDefaultRouteTag,
    ContactDetailRouteTag, ConvertedLeadDetailRouteTag, DealDefaultRouteTag,
    DealDetailRouteTag, FailedLeadDetailRouteTag, LeadDefaultRouteTag,
    LeadDetailRouteTag,
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

pub fn lead_edit_crumbs(tab: &str, name: &str, detail_url: &str) -> Markup {
    entity_crumbs("Leads", &leads_tab_url(tab), name, detail_url, Some("Edit"))
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

pub fn deals_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Deals",
        href: None,
    }])
}

pub fn deal_crumbs(name: &str, id: i64, action: Option<&str>) -> Markup {
    let list_url = DealDefaultRouteTag.url();
    entity_crumbs(
        "Deals",
        &list_url,
        name,
        &DealDetailRouteTag::new(id).url(),
        action,
    )
}
