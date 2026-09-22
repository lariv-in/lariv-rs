//! Breadcrumb trails for tasks pages.

use maud::Markup;

use crate::components::{Crumb, breadcrumbs};

use super::routes::{
    TaskDefaultRouteTag, TaskDetailRouteTag, TaskStatusDefaultRouteTag, TaskStatusDetailRouteTag,
};

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

pub fn tasks_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Tasks",
        href: None,
    }])
}

pub fn task_crumbs(name: &str, id: i64, action: Option<&str>) -> Markup {
    let list_url = TaskDefaultRouteTag.url();
    entity_crumbs(
        "Tasks",
        &list_url,
        name,
        &TaskDetailRouteTag::new(id).url(),
        action,
    )
}

pub fn task_log_crumbs(name: &str, task_id: i64, log_label: &str) -> Markup {
    breadcrumbs(&[
        Crumb {
            label: "Tasks",
            href: Some(&TaskDefaultRouteTag.url()),
        },
        Crumb {
            label: name,
            href: Some(&TaskDetailRouteTag::new(task_id).url()),
        },
        Crumb {
            label: log_label,
            href: None,
        },
    ])
}

pub fn statuses_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Statuses",
        href: None,
    }])
}

pub fn status_crumbs(name: &str, id: i64, action: Option<&str>) -> Markup {
    let list_url = TaskStatusDefaultRouteTag.url();
    entity_crumbs(
        "Statuses",
        &list_url,
        name,
        &TaskStatusDetailRouteTag::new(id).url(),
        action,
    )
}
