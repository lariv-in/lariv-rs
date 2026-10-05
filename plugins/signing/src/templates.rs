//! Signature creator modal.

use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
        ButtonSubmit, FormOpts, ShellChrome, button_submit, form, form_hx_post_url, modal_keyed,
    };
use lariv_core::html_form::CsrfToken;
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{RenderTemplate, TemplateCapability, TemplateOf, TemplateRegistrar};
use lariv_core::web::modal_create_post_query;

use super::keys::SignatureCreateModalKey;
use super::routes::SignatureCreatePostRouteTag;

lariv_core::define_register_items! {
    plugin: SigningTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        SignatureCreateModalIdx: SignatureCreateModalPageTag => SignatureCreateModalPage,
    ]
}

#[derive(Generic)]
pub struct SignatureCreateModalPage {
    pub document_id: i64,
    pub debug_local: bool,
    pub error: String,
}

impl RenderTemplate for SignatureCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let mut post_url = modal_create_post_query(
            SignatureCreatePostRouteTag,
            "p_signing.SignatureCreateForm",
            "",
            "",
        );
        if self.document_id > 0 {
            post_url.push_str(&format!("&document={}", self.document_id));
        }
        modal_keyed::<SignatureCreateModalKey>(
            "",
            html! {
                h3 class="font-bold text-lg mb-4" { "Create signature" }
                p class="mb-4" { "Generate a signing key for your account. It is used only to sign PDFs you choose." }
                @if self.debug_local {
                    p class="mb-4 text-warning" { "This server stores signing keys on disk. Use that backend only for debugging." }
                }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<SignatureCreateModalKey>(&post_url),
                    form_error: Some(self.error.as_str()).filter(|err| !err.is_empty()),
                    actions: button_submit(ButtonSubmit { label: "Generate", ..Default::default() }),
                    ..Default::default()
                }))
            },
        )
    }
}
