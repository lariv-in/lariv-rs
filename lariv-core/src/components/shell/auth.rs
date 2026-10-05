//! Auth shell: centered card inside ShellBase.

use maud::Markup;

use crate::components::layout::layout_card;
use crate::components::shell::base::{ShellBase, shell_base};

pub struct ShellAuth<'a> {
    pub title: &'a str,
    pub registry_head: Markup,
    pub extra_head: Markup,
    pub body: Markup,
    pub global_error: Option<&'a str>,
}

impl Default for ShellAuth<'_> {
    fn default() -> Self {
        Self {
            title: "Lariv",
            registry_head: Markup::default(),
            extra_head: Markup::default(),
            body: Markup::default(),
            global_error: None,
        }
    }
}

pub fn shell_auth(opts: ShellAuth<'_>) -> Markup {
    shell_base(ShellBase {
        title: opts.title,
        registry_head: opts.registry_head,
        extra_head: opts.extra_head,
        body: layout_card(opts.body),
        global_error: opts.global_error,
    })
}
