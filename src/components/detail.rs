//! Detail panel wrapper.

use maud::{Markup, html};

pub struct Detail {
    pub children: Markup,
}

pub fn detail(children: Markup) -> Markup {
    html! { div { (children) } }
}
