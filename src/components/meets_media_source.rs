//! Local camera / mic / screen preview and MoQ uplink.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::escape_attr;

const BOOTSTRAP: &str = include_str!("meets/client.js");

/// Local capture controls and preview.
pub struct MeetsMediaSource<'a> {
    pub room_code: &'a str,
    pub preview_only: bool,
    pub show_screen_share: bool,
    pub relay_url: &'a str,
    pub moq_jwt: &'a str,
    pub joined_user_id: i64,
}

impl<'a> MeetsMediaSource<'a> {
    pub fn show_screen_share(&self) -> bool {
        self.show_screen_share || !self.preview_only
    }
}

fn bootstrap() -> Markup {
    html! {
        script type="module" { (PreEscaped(BOOTSTRAP)) }
    }
}

fn mount_init() -> String {
    escape_attr(
        "(async () => { if (!window.LarivMeets) { await new Promise((r) => { const done = () => { if (window.LarivMeets) r(); }; window.addEventListener('lariv-meets-ready', done, { once: true }); done(); const id = setInterval(() => { if (window.LarivMeets) { clearInterval(id); r(); } }, 20); }); } await window.LarivMeets.mountSource($el); })()",
    )
}

/// Device picker, local `<video>` preview, mute/screen-share, and optional publish.
pub fn meets_media_source(opts: MeetsMediaSource<'_>) -> Markup {
    let x_init = mount_init();
    let preview_attr = if opts.preview_only {
        " data-preview-only"
    } else {
        ""
    };
    let jwt_attr = if opts.moq_jwt.is_empty() {
        String::new()
    } else {
        format!(r#" data-moq-jwt="{}""#, escape_attr(opts.moq_jwt))
    };
    html! {
        (bootstrap())
        (PreEscaped(format!(
            r#"<div class="meets-source flex flex-col gap-3" data-meets-source data-room-code="{code}" data-relay-url="{relay}" data-joined-user-id="{uid}"{jwt}{preview} x-data x-init="{init}">"#,
            code = escape_attr(opts.room_code),
            relay = escape_attr(opts.relay_url),
            uid = opts.joined_user_id,
            jwt = jwt_attr,
            preview = preview_attr,
            init = x_init,
        )))
        video data-local-preview class="w-full max-w-md rounded-lg bg-base-300 aspect-video object-cover" playsinline muted autoplay {}
        p data-media-error class="text-sm text-error hidden" {}
        div class="flex flex-wrap gap-2 items-end" {
            label class="form-control" {
                span class="label-text" { "Camera" }
                select data-video-device class="select select-bordered select-sm" {}
            }
            label class="form-control" {
                span class="label-text" { "Microphone" }
                select data-audio-device class="select select-bordered select-sm" {}
            }
            button type="button" class="btn btn-sm btn-outline" data-mute-audio { "Mute mic" }
            button type="button" class="btn btn-sm btn-outline" data-mute-video { "Stop camera" }
            @if opts.show_screen_share() {
                button type="button" class="btn btn-sm btn-outline" data-share-screen { "Share screen" }
            }
        }
        (PreEscaped("</div>"))
    }
}
