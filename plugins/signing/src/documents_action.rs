//! Buttons the signing plugin adds to a document detail page.

use maud::{Markup, html};

use lariv_core::components::{ButtonModalForm, SwapKey, button_modal_form};
use lariv_core::html_form::{CsrfToken, csrf_hidden_field};

use super::keys::SignatureCreateModalKey;
use super::routes::{SignDocumentPostRouteTag, SignatureCreateGetRouteTag};
use super::scope::find_own_signature;
use lariv_plugin_documents::detail_actions::{
    DocumentDetailAction, DocumentDetailActionInput, register_document_detail_action,
};
use crate::pdf::is_pdf_name;

pub struct SigningDocumentAction;

pub fn register() {
    register_document_detail_action(&SigningDocumentAction);
}

#[async_trait::async_trait]
impl DocumentDetailAction for SigningDocumentAction {
    fn id(&self) -> &'static str {
        "signing"
    }

    async fn render(&self, input: &DocumentDetailActionInput<'_>) -> Markup {
        if !is_pdf_name(input.vnode_name) {
            return Markup::default();
        }
        if find_own_signature(input.db, input.auth).await.is_some() {
            sign_form(input.document_id)
        } else {
            create_button(input.document_id)
        }
    }
}

fn create_button(document_id: i64) -> Markup {
    let href = format!(
        "{}?document={document_id}",
        SignatureCreateGetRouteTag.path()
    );
    button_modal_form(ButtonModalForm {
        name: "p_signing.SignatureCreateForm",
        href: &href,
        form_post_url: "",
        modal_uid: SignatureCreateModalKey::ID,
        label: "Sign",
        classes: "btn-outline",
        ..Default::default()
    })
}

fn sign_form(document_id: i64) -> Markup {
    let action = SignDocumentPostRouteTag::new(document_id).path();
    html! {
        form method="post" action=(action) hx-boost="false" class="inline" {
            (csrf_hidden_field(&CsrfToken::current()))
            button type="submit" class="btn btn-outline" { "Sign" }
        }
    }
}
