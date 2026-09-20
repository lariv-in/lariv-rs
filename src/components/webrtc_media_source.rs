//! Local camera / mic / screen preview and upload into the SFU peer connection.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::escape_attr;

const BOOTSTRAP: &str = include_str!("webrtc/meets.js");

/// Local capture controls and preview.
pub struct WebrtcMediaSource<'a> {
    pub room_code: &'a str,
    pub preview_only: bool,
    pub signaling_url: &'a str,
    pub joined_user_id: i64,
    pub ice_servers_json: &'a str,
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
pub fn webrtc_media_source(opts: WebrtcMediaSource<'_>) -> Markup {
    let x_init = mount_init();
    let preview_attr = if opts.preview_only {
        " data-preview-only"
    } else {
        ""
    };
    html! {
        (bootstrap())
        (PreEscaped(format!(
            r#"<div class="meets-source flex flex-col gap-3" data-webrtc-source data-room-code="{code}" data-signaling-url="{sig}" data-joined-user-id="{uid}" data-ice-servers="{ice}"{preview} x-data x-init="{init}">"#,
            code = escape_attr(opts.room_code),
            sig = escape_attr(opts.signaling_url),
            uid = opts.joined_user_id,
            ice = escape_attr(opts.ice_servers_json),
            preview = preview_attr,
            init = x_init,
        )))
        video data-local-preview class="w-full max-w-md rounded-lg bg-base-300 aspect-video object-cover" playsinline muted autoplay {}
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
            @if !opts.preview_only {
                button type="button" class="btn btn-sm btn-outline" data-share-screen { "Share screen" }
            }
        }
        (PreEscaped("</div>"))
    }
}
