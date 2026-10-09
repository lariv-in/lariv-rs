//! Breadcrumb trails for inventory pages.

use maud::Markup;

use lariv_core::components::{Crumb, breadcrumbs};

use super::routes::{
    MovementDefaultRouteTag, MovementDetailRouteTag, StockDefaultRouteTag, StockDetailRouteTag,
};

fn entity_crumbs(list_label: &'static str, list_url: &str, name: &str, detail_url: &str) -> Markup {
    breadcrumbs(&[
        Crumb {
            label: list_label,
            href: Some(list_url),
        },
        Crumb {
            label: name,
            href: Some(detail_url),
        },
    ])
}

pub fn stocks_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Stocks",
        href: None,
    }])
}

pub fn stock_crumbs(name: &str, id: i64) -> Markup {
    entity_crumbs(
        "Stocks",
        &StockDefaultRouteTag.url(),
        name,
        &StockDetailRouteTag::new(id).url(),
    )
}

pub fn movements_list_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Movements",
        href: None,
    }])
}

pub fn preferences_crumbs() -> Markup {
    breadcrumbs(&[Crumb {
        label: "Preferences",
        href: None,
    }])
}

pub fn movement_crumbs(label: &str, id: i64) -> Markup {
    entity_crumbs(
        "Movements",
        &MovementDefaultRouteTag.url(),
        label,
        &MovementDetailRouteTag::new(id).url(),
    )
}
