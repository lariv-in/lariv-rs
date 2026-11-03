use maud::{Markup, PreEscaped, html};

use lariv_core::components::{
    InputForeignKey, RenderSlot, SlotCtx, form_hx_boost_post_main, input_foreign_key,
};
use lariv_core::html_form::{CsrfToken, csrf_hidden_field};

use crate::routes::{UsersEffectiveUserGetRouteTag, UsersEffectiveUserPostRouteTag};

/// User-menu control that lets a superuser choose who the request acts as.
#[derive(Default)]
pub struct EffectiveUserSelector;

const EFFECTIVE_USER_FIELD: &str = "effective_user";

impl RenderSlot for EffectiveUserSelector {
    fn render_slot(&self, ctx: &SlotCtx) -> Markup {
        let show = crate::role_authorization::actual_auth()
            .is_some_and(|auth| crate::roles::Superuser::matches(&auth.role));
        if !show {
            return Markup::default();
        }
        let effective = crate::role_authorization::current_auth();
        let value = effective
            .as_ref()
            .map(|auth| auth.user.id.to_string())
            .unwrap_or_default();
        let display = ctx
            .name
            .clone()
            .filter(|name| !name.is_empty())
            .or_else(|| effective.map(|auth| auth.user.name))
            .unwrap_or_default();
        let action = UsersEffectiveUserPostRouteTag.path();
        let attrs = form_hx_boost_post_main(&action);
        let on_select = "const d=$event.detail; if(!d||String(d.name)!=='effective_user') return; const form=$el.closest('form'); if(!form) return; $nextTick(()=>{ if(window.htmx) window.htmx.trigger(form,'submit'); else form.requestSubmit(); })";
        html! {
            (PreEscaped(format!(
                r##"<form class="effective-user-selector w-full"{}>"##,
                attrs.as_string()
            )))
            (csrf_hidden_field(&CsrfToken::current()))
            div "@fk-select.window"=(on_select) {
                (input_foreign_key(InputForeignKey {
                    name: EFFECTIVE_USER_FIELD,
                    value: &value,
                    display: &display,
                    placeholder: "Switch user",
                    url: &UsersEffectiveUserGetRouteTag.path(),
                    uid: "effective-user",
                    classes: "w-full",
                    ..Default::default()
                }))
            }
            (PreEscaped("</form>"))
        }
    }
}
