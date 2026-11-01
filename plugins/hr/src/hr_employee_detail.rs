//! Extra employee detail-menu links contributed by HR addon plugins.
//!
//! Addons register links through
//! [`HrSidebarRegistrar::register_employee_detail_links`](crate::hr_sidebar::HrSidebarRegistrar::register_employee_detail_links)
//! on the shared HR sidebar capability hook. The folded registry is stored once at app mount.

use std::sync::OnceLock;

static LINKS: OnceLock<Vec<HrEmployeeDetailLink>> = OnceLock::new();

/// One extra entry in the employee detail sidebar (after the built-in "Detail" item).
#[derive(Clone, Debug)]
pub struct HrEmployeeDetailLink {
    /// Active-state key compared against the `active` argument of `employee_detail_menu`.
    pub key: &'static str,
    pub title: &'static str,
    /// Builds the link target for one employee id.
    pub url_fn: fn(employee_id: i64) -> String,
    pub order: u16,
}

/// Employee detail link registry folded from base + addon hooks.
#[derive(Clone, Debug, Default)]
pub struct HrEmployeeDetailRegistry {
    links: Vec<HrEmployeeDetailLink>,
}

impl HrEmployeeDetailRegistry {
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    /// Add a link; a duplicate `key` is ignored.
    pub fn push(mut self, link: HrEmployeeDetailLink) -> Self {
        if !self.links.iter().any(|l| l.key == link.key) {
            self.links.push(link);
        }
        self
    }

    pub fn links(&self) -> &[HrEmployeeDetailLink] {
        &self.links
    }

    pub(crate) fn sorted_links(self) -> Vec<HrEmployeeDetailLink> {
        let mut links = self.links;
        links.sort_by_key(|l| (l.order, l.title));
        links
    }
}

pub(crate) fn store_hr_employee_detail_links(registry: &HrEmployeeDetailRegistry) {
    if LINKS.set(registry.clone().sorted_links()).is_err() {
        tracing::error!("hr employee detail LINKS already initialized");
    }
}

/// Registered employee detail links, sorted by order.
///
/// Before app mount (unit tests, pre-mount rendering) this falls back to the base hook's links.
pub fn employee_detail_links() -> Vec<HrEmployeeDetailLink> {
    match LINKS.get() {
        Some(links) => links.clone(),
        None => {
            use crate::hr_sidebar::{BaseHook, HrSidebarRegistrar};
            BaseHook
                .register_employee_detail_links(HrEmployeeDetailRegistry::new())
                .sorted_links()
        }
    }
}
