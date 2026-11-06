//! Modals for destructive actions and state-transition confirmations.

use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_core::components::{
    ButtonSubmit, DeleteConfirmation, FieldLink, FieldText, FormOpts, ShellChrome, button_submit,
    container_column, delete_confirmation, field_link, field_text, form, form_hx_post_main_url,
    form_hx_post_route, form_hx_post_selector, label, modal, modal_keyed,
};
use lariv_core::html_form::{CsrfToken, csrf_hidden_field};
use lariv_core::template::RenderTemplate;

use crate::keys::{
    DRAFT_INVOICE_BULK_DELETE_MODAL, DRAFT_INVOICE_DELETE_MODAL, POSTED_INVOICE_RESET_DRAFT_MODAL,
};
use crate::routes::{
    DraftInvoiceBulkDeletePostRouteTag, DraftInvoiceDeletePostRouteTag,
    PostedInvoiceResetDraftRouteTag,
};

#[derive(Clone)]
pub struct ResetToDraftDeleteItem {
    pub kind: String,
    pub label: String,
    pub url: String,
}

/// Confirm resetting a posted invoice: copy it with New Draft, then cascade-delete its journal.
pub fn posted_reset_to_draft_modal(
    id: i64,
    items: &[ResetToDraftDeleteItem],
    error: Option<&str>,
    can_submit: bool,
) -> Markup {
    let body = if !can_submit {
        let err = error.unwrap_or("Reset to draft is unavailable.");
        html! {
            h3 class="font-bold text-lg" { "Reset to draft unavailable" }
            div class="alert alert-error my-2 text-sm" { (err) }
        }
    } else {
        let item_list = html! {
            p class="my-2" {
                "A new draft invoice is created from this invoice, with the same fields as New quotation. The following records are then permanently deleted:"
            }
            (container_column("", html! {
                @for item in items {
                    (label(&item.kind, reset_to_draft_item_cell(&item.label, &item.url)))
                }
            }))
        };
        form(
            &CsrfToken::current(),
            FormOpts {
                title: "Reset to draft",
                subtitle: "This cannot be undone.",
                attrs: form_hx_post_route(PostedInvoiceResetDraftRouteTag::new(id)),
                form_error: error,
                inputs: item_list,
                actions: html! {
                    (button_submit(ButtonSubmit {
                        label: "Reset to draft",
                        classes: "btn-error",
                        ..Default::default()
                    }))
                },
                ..Default::default()
            },
        )
    };
    modal_keyed(POSTED_INVOICE_RESET_DRAFT_MODAL, "", body)
}

fn reset_to_draft_item_cell(name: &str, url: &str) -> Markup {
    if url.is_empty() {
        field_text(FieldText {
            value: name,
            classes: "",
        })
    } else {
        field_link(FieldLink {
            href: url,
            label: name,
            classes: "",
        })
    }
}

/// Confirmation dialog for an invoice state transition (post, new draft).
pub struct ConfirmInvoiceTransition<'a> {
    pub modal_uid: &'a str,
    pub title: &'a str,
    pub message: &'a str,
    pub submit_label: &'a str,
    pub post_url: &'a str,
    pub can_submit: bool,
}

pub fn confirm_invoice_transition(opts: ConfirmInvoiceTransition<'_>) -> Markup {
    let onclick = format!("document.getElementById('{}').remove()", opts.modal_uid);
    let form_attrs = form_hx_post_main_url(opts.post_url);
    modal(lariv_core::components::Modal {
        uid: opts.modal_uid,
        classes: "!max-w-lg",
        children: html! {
            div class="flex flex-col gap-2" {
                h2 class="text-xl font-bold" { (opts.title) }
                p class="my-2" { (opts.message) }
                div class="flex justify-end gap-2 mt-2" {
                    button type="button" class="btn btn-ghost" onclick=(onclick) { "Cancel" }
                    @if opts.can_submit {
                        (PreEscaped(format!(
                            r#"<form class="contents"{}>"#,
                            form_attrs.as_string(),
                        )))
                        (button_submit(ButtonSubmit {
                            label: opts.submit_label,
                            ..Default::default()
                        }))
                        (PreEscaped("</form>"))
                    }
                }
            }
        },
        ..Default::default()
    })
}

#[derive(Generic)]
pub struct ConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub id: i64,
    /// Empty posts to the draft-invoice delete route.
    pub post_url: String,
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = if self.modal_uid.is_empty() {
            format!("#{}", DRAFT_INVOICE_DELETE_MODAL.id())
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            DRAFT_INVOICE_DELETE_MODAL.id()
        } else {
            self.modal_uid.as_str()
        };
        let post_url = if self.post_url.is_empty() {
            DraftInvoiceDeletePostRouteTag::new(self.id).url()
        } else {
            self.post_url.clone()
        };
        modal(lariv_core::components::Modal {
            uid,
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: &self.message,
                attrs: form_hx_post_selector(&post_url, &target),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

#[derive(Generic)]
pub struct ConfirmBulkDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub ids: String,
    /// Empty posts to the draft-invoice bulk-delete route.
    pub post_url: String,
    pub error: String,
    pub can_submit: bool,
}

impl RenderTemplate for ConfirmBulkDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = if self.modal_uid.is_empty() {
            format!("#{}", DRAFT_INVOICE_BULK_DELETE_MODAL.id())
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            DRAFT_INVOICE_BULK_DELETE_MODAL.id()
        } else {
            self.modal_uid.as_str()
        };
        let post_url = if self.post_url.is_empty() {
            DraftInvoiceBulkDeletePostRouteTag.url()
        } else {
            self.post_url.clone()
        };
        let form_attrs = form_hx_post_selector(&post_url, &target);
        modal(lariv_core::components::Modal {
            uid,
            children: html! {
                div class="container mx-auto" {
                    h2 class="text-xl font-bold text-error" { "Confirm Deletion" }
                    p class="my-2" { (self.message) }
                    @if !self.error.is_empty() {
                        div class="alert alert-error my-2 text-sm" { (self.error) }
                    }
                    @if self.can_submit {
                        (PreEscaped(format!(
                            r#"<form class="flex flex-col gap-2 my-4"{}>"#,
                            form_attrs.as_string(),
                        )))
                        (csrf_hidden_field(&CsrfToken::current()))
                        input type="hidden" name="ids" value=(self.ids);
                        div class="my-2" {
                            (button_submit(ButtonSubmit {
                                label: "Confirm Delete",
                                classes: "btn-error my-2",
                                ..Default::default()
                            }))
                        }
                        (PreEscaped("</form>"))
                    }
                }
            },
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod transition_confirm_tests {
    use super::ConfirmInvoiceTransition;
    use super::confirm_invoice_transition;

    #[test]
    fn state_transition_confirm_is_a_dialog() {
        let html = confirm_invoice_transition(ConfirmInvoiceTransition {
            modal_uid: "draft-invoice-post-modal",
            title: "Post invoice",
            message: "Post this draft invoice? This will create a posted invoice.",
            submit_label: "Post invoice",
            post_url: "/dashboard/finance-invoices/i/4/post",
            can_submit: true,
        })
        .into_string();

        assert!(html.contains(r#"<dialog id="draft-invoice-post-modal""#));
        assert!(html.contains("Post this draft invoice? This will create a posted invoice."));
        assert!(html.contains(r#"hx-post="/dashboard/finance-invoices/i/4/post""#));
        assert!(html.contains(">Post invoice</button>"));
        assert!(html.contains(">Cancel</button>"));
        assert!(!html.contains("hx-confirm"));
        assert!(!html.contains("confirm("));
    }
}
