//! Topbar-only shell (no left sidebar).

use maud::{Markup, PreEscaped, html};

use crate::components::layout::{LayoutTopbar, layout_topbar};
use crate::components::shell::base::{ShellBase, shell_base};
use crate::components::swap::{AppLayoutKey, SwapKey};

pub struct ShellTopbar<'a> {
    pub title: &'a str,
    pub registry_head: Markup,
    pub extra_head: Markup,
    pub topbar_items: Markup,
    pub body: Markup,
    pub global_error: Option<&'a str>,
}

impl Default for ShellTopbar<'_> {
    fn default() -> Self {
        Self {
            title: "Lariv",
            registry_head: Markup::default(),
            extra_head: Markup::default(),
            topbar_items: Markup::default(),
            body: Markup::default(),
            global_error: None,
        }
    }
}

pub fn shell_topbar(opts: ShellTopbar<'_>) -> Markup {
    let body = html! {
        (PreEscaped(format!(
            r#"<div id="{}" class="size-full overflow-y-auto p-4">"#,
            AppLayoutKey::ID
        )))
        (opts.body)
        (PreEscaped("</div>"))
    };
    shell_base(ShellBase {
        title: opts.title,
        registry_head: opts.registry_head,
        extra_head: opts.extra_head,
        body: layout_topbar(LayoutTopbar {
            topbar_items: opts.topbar_items,
            content: body,
            has_sidebar: false,
            x_data: None,
            right_panels: Markup::default(),
        }),
        global_error: opts.global_error,
    })
}
