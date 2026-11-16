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
    let style = format!("--heroicon-url: url('{}')", iconify_svg_url(name));
    PreEscaped(format!(
        r#"<span class="{}" style="{}"{}></span>"#,
        escape_attr(&class),
        escape_attr(&style),
        attrs.as_string()
    ))
}

/// Iconify SVG for a tile icon.
///
/// A bare name stays on the Heroicons set (`users`). `set:name` selects another
/// Iconify collection (`lucide:scroll-text`).
fn iconify_svg_url(name: &str) -> String {
    let (set, icon_name) = match name.split_once(':') {
        Some((set, icon_name)) if !set.is_empty() && !icon_name.is_empty() => (set, icon_name),
        _ => ("heroicons", name),
    };
    format!(
        "https://api.iconify.design/{}/{}.svg",
        iconify_slug(set),
        iconify_slug(icon_name)
    )
}

fn iconify_slug(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::iconify_svg_url;

    #[test]
    fn bare_names_stay_on_heroicons() {
        assert_eq!(
            iconify_svg_url("document-text"),
            "https://api.iconify.design/heroicons/document-text.svg"
        );
    }

    #[test]
    fn prefixed_names_select_the_iconify_collection() {
        assert_eq!(
            iconify_svg_url("lucide:scroll-text"),
            "https://api.iconify.design/lucide/scroll-text.svg"
        );
    }
}
