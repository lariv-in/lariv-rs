//! Record payment modal for a posted invoice.

use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonSubmit, FormOpts, ManyToManyItem, ShellChrome, button_submit, container_row, form,
    form_hx_post_url, modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::RenderTemplate;
use lariv_core::web::modal_create_post_url;

use crate::forms::{InvoicePaymentForm, InvoicePaymentFormField};
use crate::keys::INVOICE_PAY_MODAL;
use crate::routes::PostedInvoicePayPostRouteTag;

#[derive(Generic)]
pub struct InvoicePayModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub form: InvoicePaymentForm,
    pub tax_items: Vec<ManyToManyItem>,
    pub error: String,
}

impl RenderTemplate for InvoicePayModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let form_name = if self.form_name.is_empty() {
            "p_finance_invoices.InvoicePaymentForm"
        } else {
            self.form_name.as_str()
        };
        modal_keyed(
            INVOICE_PAY_MODAL,
            "!max-w-6xl w-full",
            form(
                &CsrfToken::current(),
                FormOpts {
                    title: "Record payment",
                    subtitle: "Settle this posted invoice",
                    classes: "@container",
                    attrs: form_hx_post_url(
                        INVOICE_PAY_MODAL,
                        &modal_create_post_url(
                            PostedInvoicePayPostRouteTag::new(self.form.posted_invoice_id),
                            form_name,
                            &self.refresh_table,
                        ),
                    ),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: InvoicePaymentForm::render_inputs(
                        &FormCtx::form::<InvoicePaymentForm>(CsrfToken::current())
                            .value(
                                InvoicePaymentFormField::PostedInvoiceId,
                                self.form.posted_invoice_id.to_string(),
                            )
                            .value(InvoicePaymentFormField::Number, &self.form.number)
                            .value(InvoicePaymentFormField::Amount, &self.form.amount)
                            .value(InvoicePaymentFormField::Datetime, &self.form.datetime)
                            .value(
                                InvoicePaymentFormField::JournalLines,
                                &self.form.journal_lines,
                            )
                            .m2m(InvoicePaymentFormField::Taxes, &self.tax_items),
                    ),
                    actions: html! {
                        (container_row("flex justify-end gap-2 mt-2", html! {
                            (button_submit(ButtonSubmit {
                                label: "Save",
                                classes: "btn-primary",
                                ..Default::default()
                            }))
                        }))
                    },
                    ..Default::default()
                },
            ),
        )
    }
}
