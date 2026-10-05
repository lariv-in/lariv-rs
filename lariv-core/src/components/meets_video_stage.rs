//! Remote participant video/audio tile grid. Must not be an HTMX swap target.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::escape_attr;

const BOOTSTRAP: &str = include_str!("meets/client.js");

/// Remote participant stage (id `meets-room-media`).
pub struct MeetsVideoStage<'a> {
    pub room_code: &'a str,
    pub relay_url: &'a str,
    pub moq_jwt: &'a str,
    pub joined_user_id: i64,
}

fn bootstrap() -> Markup {
    html! {
        script type="module" { (PreEscaped(BOOTSTRAP)) }
    }
}

fn mount_init(kind: &str) -> String {
    escape_attr(&format!(
        "(async () => {{ if (!window.LarivMeets) {{ await new Promise((r) => {{ const done = () => {{ if (window.LarivMeets) r(); }}; window.addEventListener('lariv-meets-ready', done, {{ once: true }}); done(); const id = setInterval(() => {{ if (window.LarivMeets) {{ clearInterval(id); r(); }} }}, 20); }}); }} await window.LarivMeets.{kind}($el); }})()"
    ))
}

/// Render the remote stream grid. HTMX must never morph this node.
pub fn meets_video_stage(opts: MeetsVideoStage<'_>) -> Markup {
    let x_init = mount_init("mountStage");
    let jwt_attr = if opts.moq_jwt.is_empty() {
        String::new()
    } else {
        format!(r#" data-moq-jwt="{}""#, escape_attr(opts.moq_jwt))
    };
    html! {
        (bootstrap())
        (PreEscaped(format!(
            r#"<div id="meets-room-media" class="meets-stage grid grid-cols-1 sm:grid-cols-2 gap-3 min-h-48" data-meets-stage data-room-code="{code}" data-relay-url="{relay}" data-joined-user-id="{uid}"{jwt} x-data x-init="{init}"><div data-remote-grid class="contents"></div></div>"#,
            code = escape_attr(opts.room_code),
            relay = escape_attr(opts.relay_url),
            uid = opts.joined_user_id,
            jwt = jwt_attr,
            init = x_init,
        )))
    }
}
