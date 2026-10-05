//! Topbar-only shell (no left sidebar).

use maud::{Markup, PreEscaped, html};

use crate::components::layout::layout_topbar_with_right_sidebar;
use crate::components::shell::base::{ShellBase, shell_base};

pub struct ShellTopbar<'a> {
    pub title: &'a str,
    pub registry_head: Markup,
    pub extra_head: Markup,
    pub topbar_items: Markup,
    /// Right drawer panel (e.g. LLM assistant history). Empty ⇒ no toggle/aside.
    pub right_sidebar: Markup,
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
            right_sidebar: Markup::default(),
            body: Markup::default(),
            global_error: None,
        }
    }
}

pub fn shell_topbar(opts: ShellTopbar<'_>) -> Markup {
    let body = html! {
        (PreEscaped(format!(
            r#"<div {} class="size-full overflow-y-auto p-4">"#,
            crate::components::swap::app_layout_history_attrs()
        )))
        (opts.body)
        (PreEscaped("</div>"))
    };
    shell_base(ShellBase {
        title: opts.title,
        registry_head: opts.registry_head,
        extra_head: opts.extra_head,
        body: layout_topbar_with_right_sidebar(opts.topbar_items, body, opts.right_sidebar),
        global_error: opts.global_error,
    })
}
