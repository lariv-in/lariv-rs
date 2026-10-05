//! Text and icon primitives.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::{HtmlAttrs, escape_attr};

pub fn escaped_string(value: &str) -> Markup {
    html! { (value) }
}

pub fn raw_string(value: &str) -> Markup {
    PreEscaped(value.to_string())
}

pub fn icon(name: &str, classes: &str) -> Markup {
    icon_with_attrs(name, classes, &HtmlAttrs::new())
}

pub fn icon_with_attrs(name: &str, classes: &str, attrs: &HtmlAttrs) -> Markup {
    let class = format!("heroicon {}", classes);
    let style = format!(
        "--heroicon-url: url('https://api.iconify.design/heroicons/{}.svg')",
        name
    );
    PreEscaped(format!(
        r#"<span class="{}" style="{}"{}></span>"#,
        escape_attr(&class),
        escape_attr(&style),
        attrs.as_string()
    ))
}
