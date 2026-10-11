//! Signed-in topbar control that subscribes the browser for task push.

use maud::{Markup, PreEscaped, html};

use lariv_core::components::{RenderSlot, SlotCtx, icon};

use crate::routes::{PushSubscribeRouteTag, PushUnsubscribeRouteTag, PushVapidRouteTag};

#[derive(Default)]
pub struct TasksPushScript;

impl RenderSlot for TasksPushScript {
    fn render_slot(&self, ctx: &SlotCtx) -> Markup {
        if ctx.name.is_none() {
            return html! {};
        }
        let js = include_str!("subscribe.js")
            .replace("__VAPID_URL__", &PushVapidRouteTag.path())
            .replace("__SUBSCRIBE_URL__", &PushSubscribeRouteTag.path())
            .replace("__UNSUBSCRIBE_URL__", &PushUnsubscribeRouteTag.path());
        html! {
            (PreEscaped(format!("<script>{js}</script>")))
        }
    }
}

#[derive(Default)]
pub struct TasksPushButton;

impl RenderSlot for TasksPushButton {
    fn render_slot(&self, ctx: &SlotCtx) -> Markup {
        if ctx.name.is_none() {
            return html! {};
        }
        html! {
            (PreEscaped(
                r#"<button type="button" id="tasks-push-enable" class="btn items-center btn-sm btn-square btn-outline" hidden aria-label="Enable task notifications">"#,
            ))
            (icon("bell", ""))
            (PreEscaped("</button>"))
        }
    }
}
