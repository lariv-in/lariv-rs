//! Remote WebRTC video/audio tile grid. Must not be an HTMX swap target.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::escape_attr;

const BOOTSTRAP: &str = include_str!("webrtc/meets.js");
const VIDEO_CODEC_PRIORITY_JSON: &str = r#"["AV1","H265","VP9","VP8","H264"]"#;

/// Remote participant stage (id `meets-room-media`).
pub struct WebrtcVideoStage<'a> {
    pub room_code: &'a str,
    pub signaling_url: &'a str,
    pub joined_user_id: i64,
    pub ice_servers_json: &'a str,
    pub video_codec_priority_json: &'a str,
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
pub fn webrtc_video_stage(opts: WebrtcVideoStage<'_>) -> Markup {
    let x_init = mount_init("mountStage");
    html! {
        (bootstrap())
        (PreEscaped(format!(
            r#"<div id="meets-room-media" class="meets-stage grid grid-cols-1 sm:grid-cols-2 gap-3 min-h-48" data-webrtc-stage data-room-code="{code}" data-signaling-url="{sig}" data-joined-user-id="{uid}" data-ice-servers="{ice}" data-video-codecs="{codecs}" x-data x-init="{init}">"#,
            code = escape_attr(opts.room_code),
            sig = escape_attr(opts.signaling_url),
            uid = opts.joined_user_id,
            ice = escape_attr(opts.ice_servers_json),
            codecs = escape_attr(if opts.video_codec_priority_json.is_empty() {
                VIDEO_CODEC_PRIORITY_JSON
            } else {
                opts.video_codec_priority_json
            }),
            init = x_init,
        )))
        div data-remote-grid class="contents" {}
        (PreEscaped("</div>"))
    }
}
