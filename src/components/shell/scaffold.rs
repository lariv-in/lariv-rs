//! Full scaffold: topbar + left sidebar.

use maud::Markup;

use crate::components::layout::{
    LayoutSidebar, layout_sidebar, layout_topbar_with_right_sidebar,
};
use crate::components::shell::base::{ShellBase, shell_base};

pub struct ShellScaffold<'a> {
    pub title: &'a str,
    pub registry_head: Markup,
    pub extra_head: Markup,
    pub topbar_items: Markup,
    pub right_sidebar: Markup,
    pub sidebar: Markup,
    pub body: Markup,
    pub global_error: Option<&'a str>,
}

impl Default for ShellScaffold<'_> {
    fn default() -> Self {
        Self {
            title: "Lariv",
            registry_head: Markup::default(),
            extra_head: Markup::default(),
            topbar_items: Markup::default(),
            right_sidebar: Markup::default(),
            sidebar: Markup::default(),
            body: Markup::default(),
            global_error: None,
        }
    }
}

pub fn shell_scaffold(opts: ShellScaffold<'_>) -> Markup {
    let content = layout_sidebar(LayoutSidebar {
        sidebar: opts.sidebar,
        content: opts.body,
    });
    shell_base(ShellBase {
        title: opts.title,
        registry_head: opts.registry_head,
        extra_head: opts.extra_head,
        body: layout_topbar_with_right_sidebar(opts.topbar_items, content, opts.right_sidebar),
        global_error: opts.global_error,
    })
}
