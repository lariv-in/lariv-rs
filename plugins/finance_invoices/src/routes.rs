use super::{
    customer_journal, handlers,
    keys::{
        DRAFT_INVOICE_BULK_DELETE_MODAL, DRAFT_INVOICE_DELETE_MODAL, DRAFT_INVOICE_SELECT_MODAL,
        DRAFT_INVOICE_SELECT_TABLE, INVOICE_HUB_TABLE, POSTED_INVOICE_RESET_DRAFT_MODAL,
        QUOTATION_INVOICE_BULK_DELETE_MODAL, QUOTATION_INVOICE_DELETE_MODAL,
        QUOTATION_INVOICE_SELECT_MODAL, QUOTATION_INVOICE_SELECT_TABLE,
    },
};

pub struct FinanceInvoicesView;

pub struct FinanceInvoicesMutate;

lariv_core::define_plugin_routes! {
    plugin: FinanceInvoicesTag;
    prefix: "/dashboard";
    routes: [
        get InvoiceDefaultRouteTag, "/finance-invoices", handlers::hub::hub, fragment(INVOICE_HUB_TABLE), authorize(FinanceInvoicesView, []);
        get CustomerJournalQuotationsRouteTag, "/finance/customers/{id}/journal/quotations", customer_journal::quotations, authorize(FinanceInvoicesView, []);
        get CustomerJournalDraftInvoicesRouteTag, "/finance/customers/{id}/journal/draft-invoices", customer_journal::draft_invoices, authorize(FinanceInvoicesView, []);
        get CustomerJournalInvoicesRouteTag, "/finance/customers/{id}/journal/invoices", customer_journal::invoices, authorize(FinanceInvoicesView, []);
        get CustomerJournalCancelledRouteTag, "/finance/customers/{id}/journal/cancelled", customer_journal::cancelled, authorize(FinanceInvoicesView, []);
        get InvoiceLinePriceRouteTag, "/finance-invoices/line-price", handlers::drafts::line_price, authorize(FinanceInvoicesView, []);
        get DraftInvoiceCreateGetRouteTag, "/finance-invoices/create", handlers::drafts::create_get, modal, authorize(FinanceInvoicesMutate, []);
        post DraftInvoiceCreatePostRouteTag, "/finance-invoices/create", handlers::drafts::create_post, authorize(FinanceInvoicesMutate, []);
        get DraftInvoiceDetailRouteTag, "/finance-invoices/i/{id}", handlers::drafts::detail, authorize(FinanceInvoicesView, []);
        get DraftInvoiceEditGetRouteTag, "/finance-invoices/i/{id}/edit", handlers::drafts::edit_get, modal, authorize(FinanceInvoicesMutate, []);
        post DraftInvoiceEditPostRouteTag, "/finance-invoices/i/{id}/edit", handlers::drafts::edit_post, authorize(FinanceInvoicesMutate, []);
        get DraftInvoiceDeleteGetRouteTag, "/finance-invoices/i/{id}/delete", handlers::drafts::delete_get, modal, authorize(FinanceInvoicesMutate, []);
        post DraftInvoiceDeletePostRouteTag, "/finance-invoices/i/{id}/delete", bare handlers::drafts::delete_post, fragment(DRAFT_INVOICE_DELETE_MODAL), authorize(FinanceInvoicesMutate, []);
        get DraftInvoicePostGetRouteTag, "/finance-invoices/i/{id}/post", handlers::drafts::post_confirm_get, modal, authorize(FinanceInvoicesMutate, []);
        post DraftInvoicePostRouteTag, "/finance-invoices/i/{id}/post", bare handlers::drafts::post_invoice, redirect, authorize(FinanceInvoicesMutate, []);
        get DraftInvoiceBulkDeleteGetRouteTag, "/finance-invoices/bulk-delete", handlers::drafts::bulk_delete_get, modal, authorize(FinanceInvoicesMutate, []);
        post DraftInvoiceBulkDeletePostRouteTag, "/finance-invoices/bulk-delete", bare handlers::drafts::bulk_delete_post, fragment(DRAFT_INVOICE_BULK_DELETE_MODAL), authorize(FinanceInvoicesMutate, []);
        get DraftInvoiceBulkEditGetRouteTag, "/finance-invoices/bulk-edit", handlers::drafts::bulk_edit_get, modal, authorize(FinanceInvoicesMutate, []);
        post DraftInvoiceBulkEditPostRouteTag, "/finance-invoices/bulk-edit", handlers::drafts::bulk_edit_post, authorize(FinanceInvoicesMutate, []);
        get DraftInvoiceBulkPostGetRouteTag, "/finance-invoices/bulk-post", handlers::drafts::bulk_post_confirm_get, modal, authorize(FinanceInvoicesMutate, []);
        post DraftInvoiceBulkPostRouteTag, "/finance-invoices/bulk-post", bare handlers::drafts::bulk_post, redirect, authorize(FinanceInvoicesMutate, []);
        get DraftInvoicePdfModalRouteTag, "/finance-invoices/i/{id}/pdf", bare handlers::pdf::draft_pdf_modal, modal, authorize(FinanceInvoicesView, []);
        get DraftInvoicePdfRouteTag, "/finance-invoices/i/{id}/pdf/file", bare handlers::pdf::draft_pdf, file, authorize(FinanceInvoicesView, []);
        get DraftInvoiceMultiSelectRouteTag, "/finance-invoices/pick", handlers::drafts::multi_select, multi_select(DRAFT_INVOICE_SELECT_TABLE, DRAFT_INVOICE_SELECT_MODAL), authorize(FinanceInvoicesView, []);
        get QuotationInvoiceCreateGetRouteTag, "/finance-invoices/quotations/create", handlers::quotations::create_get, modal, authorize(FinanceInvoicesMutate, []);
        post QuotationInvoiceCreatePostRouteTag, "/finance-invoices/quotations/create", handlers::quotations::create_post, authorize(FinanceInvoicesMutate, []);
        get QuotationInvoiceBulkDeleteGetRouteTag, "/finance-invoices/quotations/bulk-delete", handlers::quotations::bulk_delete_get, modal, authorize(FinanceInvoicesMutate, []);
        post QuotationInvoiceBulkDeletePostRouteTag, "/finance-invoices/quotations/bulk-delete", bare handlers::quotations::bulk_delete_post, fragment(QUOTATION_INVOICE_BULK_DELETE_MODAL), authorize(FinanceInvoicesMutate, []);
        get QuotationInvoiceBulkEditGetRouteTag, "/finance-invoices/quotations/bulk-edit", handlers::quotations::bulk_edit_get, modal, authorize(FinanceInvoicesMutate, []);
        post QuotationInvoiceBulkEditPostRouteTag, "/finance-invoices/quotations/bulk-edit", handlers::quotations::bulk_edit_post, authorize(FinanceInvoicesMutate, []);
        get QuotationInvoiceBulkCreateDraftGetRouteTag, "/finance-invoices/quotations/bulk-create-draft", handlers::quotations::bulk_create_draft_get, modal, authorize(FinanceInvoicesMutate, []);
        post QuotationInvoiceBulkCreateDraftRouteTag, "/finance-invoices/quotations/bulk-create-draft", bare handlers::quotations::bulk_create_draft_post, redirect, authorize(FinanceInvoicesMutate, []);
        get QuotationInvoiceMultiSelectRouteTag, "/finance-invoices/quotations/pick", handlers::quotations::multi_select, multi_select(QUOTATION_INVOICE_SELECT_TABLE, QUOTATION_INVOICE_SELECT_MODAL), authorize(FinanceInvoicesView, []);
        get QuotationInvoiceDetailRouteTag, "/finance-invoices/quotations/{id}", handlers::quotations::detail, authorize(FinanceInvoicesView, []);
        get QuotationInvoiceEditGetRouteTag, "/finance-invoices/quotations/{id}/edit", handlers::quotations::edit_get, modal, authorize(FinanceInvoicesMutate, []);
        post QuotationInvoiceEditPostRouteTag, "/finance-invoices/quotations/{id}/edit", handlers::quotations::edit_post, authorize(FinanceInvoicesMutate, []);
        get QuotationInvoiceDeleteGetRouteTag, "/finance-invoices/quotations/{id}/delete", handlers::quotations::delete_get, modal, authorize(FinanceInvoicesMutate, []);
        post QuotationInvoiceDeletePostRouteTag, "/finance-invoices/quotations/{id}/delete", bare handlers::quotations::delete_post, fragment(QUOTATION_INVOICE_DELETE_MODAL), authorize(FinanceInvoicesMutate, []);
        get QuotationInvoiceCreateDraftGetRouteTag, "/finance-invoices/quotations/{id}/create-draft", handlers::quotations::create_draft_get, modal, authorize(FinanceInvoicesMutate, []);
        post QuotationInvoiceCreateDraftRouteTag, "/finance-invoices/quotations/{id}/create-draft", bare handlers::quotations::create_draft_post, redirect, authorize(FinanceInvoicesMutate, []);
        get QuotationInvoicePdfModalRouteTag, "/finance-invoices/quotations/{id}/pdf", bare handlers::pdf::quotation_pdf_modal, modal, authorize(FinanceInvoicesView, []);
        get QuotationInvoicePdfRouteTag, "/finance-invoices/quotations/{id}/pdf/file", bare handlers::pdf::quotation_pdf, file, authorize(FinanceInvoicesView, []);

        get PostedInvoiceDetailRouteTag, "/finance-invoices/posted/{id}", handlers::posted::detail, authorize(FinanceInvoicesView, []);
        get PostedInvoiceCancelGetRouteTag, "/finance-invoices/posted/{id}/cancel", handlers::posted::cancel_get, authorize(FinanceInvoicesMutate, []);
        post PostedInvoiceCancelRouteTag, "/finance-invoices/posted/{id}/cancel", bare handlers::posted::cancel_invoice, redirect, authorize(FinanceInvoicesMutate, []);
        get PostedInvoiceNewDraftGetRouteTag, "/finance-invoices/posted/{id}/new-draft", handlers::posted::new_draft_confirm_get, modal, authorize(FinanceInvoicesMutate, []);
        post PostedInvoiceNewDraftRouteTag, "/finance-invoices/posted/{id}/new-draft", bare handlers::posted::new_draft, redirect, authorize(FinanceInvoicesMutate, []);
        get PostedInvoiceResetDraftGetRouteTag, "/finance-invoices/posted/{id}/reset-draft", handlers::posted::reset_draft_get, modal, authorize(FinanceInvoicesMutate, []);
        post PostedInvoiceResetDraftRouteTag, "/finance-invoices/posted/{id}/reset-draft", bare handlers::posted::reset_draft_post, fragment(POSTED_INVOICE_RESET_DRAFT_MODAL), authorize(FinanceInvoicesMutate, []);
        get PostedInvoiceBulkNewDraftGetRouteTag, "/finance-invoices/posted/bulk-new-draft", handlers::posted::bulk_new_draft_confirm_get, modal, authorize(FinanceInvoicesMutate, []);
        post PostedInvoiceBulkNewDraftRouteTag, "/finance-invoices/posted/bulk-new-draft", bare handlers::posted::bulk_new_draft, redirect, authorize(FinanceInvoicesMutate, []);
        get PostedInvoiceBulkCancelGetRouteTag, "/finance-invoices/bulk-cancel", handlers::posted::bulk_cancel_get, authorize(FinanceInvoicesMutate, []);
        post PostedInvoiceBulkCancelPostRouteTag, "/finance-invoices/bulk-cancel", bare handlers::posted::bulk_cancel_post, redirect, authorize(FinanceInvoicesMutate, []);
        get PostedInvoicePdfModalRouteTag, "/finance-invoices/posted/{id}/pdf", bare handlers::pdf::posted_pdf_modal, modal, authorize(FinanceInvoicesView, []);
        get PostedInvoicePdfRouteTag, "/finance-invoices/posted/{id}/pdf/file", bare handlers::pdf::posted_pdf, file, authorize(FinanceInvoicesView, []);

        get PostedInvoicePayGetRouteTag, "/finance-invoices/posted/{id}/pay", handlers::pay::pay_get, modal, authorize(FinanceInvoicesMutate, []);
        post PostedInvoicePayPostRouteTag, "/finance-invoices/posted/{id}/pay", handlers::pay::pay_post, authorize(FinanceInvoicesMutate, []);
        post InvoicePayJournalLinesRouteTag, "/finance-invoices/pay-journal-lines", bare handlers::pay::fill_journal_lines, raw, authorize(FinanceInvoicesMutate, []);

        get CancelledInvoiceDetailRouteTag, "/finance-invoices/cancelled/{id}", handlers::cancelled::detail, authorize(FinanceInvoicesView, []);
        get CancelledInvoiceNewDraftGetRouteTag, "/finance-invoices/cancelled/{id}/new-draft", handlers::cancelled::new_draft_confirm_get, modal, authorize(FinanceInvoicesMutate, []);
        post CancelledInvoiceNewDraftRouteTag, "/finance-invoices/cancelled/{id}/new-draft", bare handlers::cancelled::new_draft, redirect, authorize(FinanceInvoicesMutate, []);
        get CancelledInvoiceBulkNewDraftGetRouteTag, "/finance-invoices/cancelled/bulk-new-draft", handlers::cancelled::bulk_new_draft_confirm_get, modal, authorize(FinanceInvoicesMutate, []);
        post CancelledInvoiceBulkNewDraftRouteTag, "/finance-invoices/cancelled/bulk-new-draft", bare handlers::cancelled::bulk_new_draft, redirect, authorize(FinanceInvoicesMutate, []);
        get CancelledInvoicePdfModalRouteTag, "/finance-invoices/cancelled/{id}/pdf", bare handlers::pdf::cancelled_pdf_modal, modal, authorize(FinanceInvoicesView, []);
        get CancelledInvoicePdfRouteTag, "/finance-invoices/cancelled/{id}/pdf/file", bare handlers::pdf::cancelled_pdf, file, authorize(FinanceInvoicesView, []);

        get PaidInvoiceDetailRouteTag, "/finance-invoices/paid/{id}", handlers::settlements::paid_detail, authorize(FinanceInvoicesView, []);
        get PartiallyPaidInvoiceDetailRouteTag, "/finance-invoices/partial/{id}", handlers::settlements::partial_detail, authorize(FinanceInvoicesView, []);

        get InvoicePreferencesRouteTag, "/finance-invoices/preferences", handlers::preferences::invoice_preferences_get, authorize(FinanceInvoicesView, []);
        post InvoicePreferencesPostRouteTag, "/finance-invoices/preferences", bare handlers::preferences::invoice_preferences_post, redirect, authorize(FinanceInvoicesMutate, []);

        get PaidInvoicePdfModalRouteTag, "/finance-invoices/paid/{id}/pdf", bare handlers::pdf::paid_pdf_modal, modal, authorize(FinanceInvoicesView, []);
        get PaidInvoicePdfRouteTag, "/finance-invoices/paid/{id}/pdf/file", bare handlers::pdf::paid_pdf, file, authorize(FinanceInvoicesView, []);
        get PartiallyPaidInvoicePdfModalRouteTag, "/finance-invoices/partial/{id}/pdf", bare handlers::pdf::partially_paid_pdf_modal, modal, authorize(FinanceInvoicesView, []);
        get PartiallyPaidInvoicePdfRouteTag, "/finance-invoices/partial/{id}/pdf/file", bare handlers::pdf::partially_paid_pdf, file, authorize(FinanceInvoicesView, []);
        get InvoiceBulkPdfsRouteTag, "/finance-invoices/bulk-pdfs", bare handlers::pdf::bulk_pdfs, file, authorize(FinanceInvoicesView, []);

        post InvoicePdfPreviewPostRouteTag, "/finance-invoices/invoice-pdf-preview", bare handlers::invoice_pdf_preview::modal_post, modal, authorize(FinanceInvoicesMutate, []);
        get InvoicePdfPreviewPdfRouteTag, "/finance-invoices/invoice-pdf-preview/{token}", bare handlers::invoice_pdf_preview::pdf_get, file, param token: String, authorize(FinanceInvoicesView, []);
    ]
}
