//! Cancel posted invoice (single and bulk) pages.

use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_core::components::{
    ButtonSubmit, FieldTitle, ShellChrome, button_submit, field_title, form_hx_post_main_url,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm, csrf_hidden_field};
use lariv_core::template::{RenderAppPane, RenderTemplate};

use lariv_plugin_finance_accounts::templates::{
    app_scaffold, app_scaffold_with_sidebar, layout_main_with_crumbs,
    layout_with_entity_sidebar_crumbs, layout_with_sidebar_crumbs,
};

use crate::forms::{CancelInvoiceForm, CancelInvoiceFormField};
use crate::routes::{
    InvoiceDefaultRouteTag, PostedInvoiceBulkCancelPostRouteTag, PostedInvoiceCancelRouteTag,
    PostedInvoiceDetailRouteTag,
};

use super::common::{
    invoice_number_label, invoice_section_crumbs, invoices_list_crumbs, posted_invoice_detail_menu,
};

#[derive(Generic)]
pub struct CancelInvoicePage {
    pub id: i64,
    pub form: CancelInvoiceForm,
    pub can_edit: bool,
}

impl CancelInvoicePage {
    fn body(&self) -> Markup {
        html! {
            (field_title(FieldTitle { value: &format!("Cancel posted invoice #{}", self.id), classes: "" }))
            form method="post" action=(PostedInvoiceCancelRouteTag::new(self.id).url()) {
                (csrf_hidden_field(&CsrfToken::current()))
                (CancelInvoiceForm::render_inputs(
                    &FormCtx::form::<CancelInvoiceForm>(CsrfToken::current())
                        .value(CancelInvoiceFormField::Reason, &self.form.reason),
                ))
                (button_submit(ButtonSubmit { label: "Cancel invoice", ..Default::default() }))
            }
        }
    }

    fn menu(&self) -> Markup {
        posted_invoice_detail_menu(self.id, "")
    }
}

impl RenderAppPane for CancelInvoicePage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let label = invoice_number_label(self.id, "");
        let detail_url = PostedInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, Some("Cancel"));
        layout_with_entity_sidebar_crumbs(self.menu(), crumbs, self.body())
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        let label = invoice_number_label(self.id, "");
        let detail_url = PostedInvoiceDetailRouteTag::new(self.id).url();
        layout_main_with_crumbs(
            invoice_section_crumbs(&label, &detail_url, Some("Cancel")),
            self.body(),
        )
    }
}

impl RenderTemplate for CancelInvoicePage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let label = invoice_number_label(self.id, "");
        let detail_url = PostedInvoiceDetailRouteTag::new(self.id).url();
        let crumbs = invoice_section_crumbs(&label, &detail_url, Some("Cancel"));
        app_scaffold_with_sidebar("Cancel Invoice", chrome, self.menu(), crumbs, self.body())
    }
}

#[derive(Generic)]
pub struct CancelBulkInvoicePage {
    pub ids: String,
    pub count: usize,
    pub form: CancelInvoiceForm,
    pub can_edit: bool,
    pub error: String,
}

impl CancelBulkInvoicePage {
    fn title(&self) -> String {
        if self.count == 1 {
            "Cancel 1 posted invoice".into()
        } else {
            format!("Cancel {} posted invoices", self.count)
        }
    }

    fn body(&self) -> Markup {
        let title = self.title();
        let post_url = PostedInvoiceBulkCancelPostRouteTag.url();
        let form_attrs = form_hx_post_main_url(&post_url);
        html! {
            (field_title(FieldTitle { value: &title, classes: "" }))
            @if !self.error.is_empty() {
                p class="text-error mb-2" { (self.error) }
            }
            @if self.count > 0 && self.can_edit {
                (PreEscaped(format!(
                    r#"<form method="POST"{}>"#,
                    form_attrs.as_string(),
                )))
                (csrf_hidden_field(&CsrfToken::current()))
                input type="hidden" name="ids" value=(self.ids);
                (CancelInvoiceForm::render_inputs(
                    &FormCtx::form::<CancelInvoiceForm>(CsrfToken::current())
                        .value(CancelInvoiceFormField::Reason, &self.form.reason),
                ))
                (button_submit(ButtonSubmit {
                    label: "Cancel invoices",
                    classes: "btn-error",
                    ..Default::default()
                }))
                (PreEscaped("</form>"))
            }
        }
    }
}

impl RenderAppPane for CancelBulkInvoicePage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        layout_with_sidebar_crumbs(
            &InvoiceDefaultRouteTag.url(),
            invoices_list_crumbs(),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        layout_main_with_crumbs(invoices_list_crumbs(), self.body())
    }
}

impl RenderTemplate for CancelBulkInvoicePage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Cancel Invoices",
            chrome,
            invoices_list_crumbs(),
            self.body(),
            &InvoiceDefaultRouteTag.url(),
        )
    }
}
