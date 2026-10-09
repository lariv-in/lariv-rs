//! Breadcrumb trails for HR pages.

use maud::Markup;

use lariv_core::components::{Crumb, breadcrumbs};

use super::routes::{
    ApplicantHubRouteTag, AttendanceListRouteTag, EmployeeDetailRouteTag, HolidayListRouteTag,
    JobFormListRouteTag,
};

fn hub_tab_url(tab: &str) -> String {
    lariv_core::http::RouteQueryBuilder::new(ApplicantHubRouteTag)
        .query("tab", tab)
        .build()
}

pub fn hub_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "People",
        href: None,
    }])
}

pub fn holidays_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Holidays",
        href: None,
    }])
}

pub fn holiday_crumbs(title: &str) -> Markup {
    let list_url = HolidayListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Holidays",
            href: Some(&list_url),
        },
        Crumb {
            label: title,
            href: None,
        },
    ])
}

pub fn attendances_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Attendance",
        href: None,
    }])
}

pub fn attendance_crumbs(title: &str) -> Markup {
    let list_url = AttendanceListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Attendance",
            href: Some(&list_url),
        },
        Crumb {
            label: title,
            href: None,
        },
    ])
}

pub fn overtime_list_crumbs(label: &str) -> Markup {
    breadcrumbs(&[Crumb { label, href: None }])
}

pub fn overtime_crumbs(parent: &str, parent_href: &str, title: &str) -> Markup {
    breadcrumbs(&[
        Crumb {
            label: parent,
            href: Some(parent_href),
        },
        Crumb {
            label: title,
            href: None,
        },
    ])
}

pub fn leave_preferences_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Leave preferences",
        href: None,
    }])
}

pub fn leaves_list_crumbs(label: &str) -> Markup {
    breadcrumbs(&[Crumb { label, href: None }])
}

pub fn leave_crumbs(parent: &str, parent_href: &str, title: &str) -> Markup {
    breadcrumbs(&[
        Crumb {
            label: parent,
            href: Some(parent_href),
        },
        Crumb {
            label: title,
            href: None,
        },
    ])
}

pub fn job_forms_crumbs(label: &str) -> Markup {
    let list_url = JobFormListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Job postings",
            href: Some(&list_url),
        },
        Crumb { label, href: None },
    ])
}

pub fn applicant_crumbs(name: &str) -> Markup {
    breadcrumbs(&[
        Crumb {
            label: "People",
            href: Some(&hub_tab_url("applicants")),
        },
        Crumb {
            label: name,
            href: None,
        },
    ])
}

pub fn employee_crumbs(name: &str) -> Markup {
    breadcrumbs(&[
        Crumb {
            label: "People",
            href: Some(&hub_tab_url("employees")),
        },
        Crumb {
            label: name,
            href: None,
        },
    ])
}

pub fn employee_leave_journal_crumbs(name: &str, employee_id: i64) -> Markup {
    let people = hub_tab_url("employees");
    let detail = EmployeeDetailRouteTag::new(employee_id).url();
    breadcrumbs(&[
        Crumb {
            label: "People",
            href: Some(&people),
        },
        Crumb {
            label: name,
            href: Some(&detail),
        },
        Crumb {
            label: "Leaves journal",
            href: None,
        },
    ])
}

pub fn ex_employee_crumbs(name: &str) -> Markup {
    breadcrumbs(&[
        Crumb {
            label: "People",
            href: Some(&hub_tab_url("ex_employees")),
        },
        Crumb {
            label: name,
            href: None,
        },
    ])
}
