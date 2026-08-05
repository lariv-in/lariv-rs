//! Detail panel wrapper.

use maud::{Markup, html};

use crate::components::field::{FieldTitle, field_title};

pub struct Detail {
    pub children: Markup,
}

pub fn detail(children: Markup) -> Markup {
    html! { div { (children) } }
}

/// Detail page title row with primary actions aligned to the top-right.
pub struct DetailHeader<'a> {
    pub title: &'a str,
    pub actions: Markup,
}

/// Render a detail page heading with optional action buttons above the field list.
pub fn detail_header(opts: DetailHeader<'_>) -> Markup {
    html! {
        div class="flex flex-wrap items-center justify-between gap-2 mb-4" {
            (field_title(FieldTitle {
                value: opts.title,
                classes: "mb-0",
            }))
            div class="flex flex-wrap items-center gap-2" {
                (opts.actions)
            }
        }
    }
}
