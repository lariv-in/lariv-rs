use super::{
    handlers,
    keys::{
        DraftInvoiceBulkDeleteModalKey, DraftInvoiceDeleteModalKey, DraftInvoiceSelectModalKey,
        DraftInvoiceSelectTableKey, InvoiceHubTableKey, PaymentTableKey,
        PostedInvoiceSelectModalKey, PostedInvoiceSelectTableKey,
    },
};

pub struct FinanceInvoicesView;

pub struct FinanceInvoicesMutate;

lariv_core::define_plugin_routes! {
    plugin: FinanceInvoicesTag;
    prefix: "/dashboard";
    routes: [
        get InvoiceDefaultRouteTag, "/finance-invoices", handlers::hub::hub, fragment(InvoiceHubTableKey), authorize(FinanceInvoicesView, []);
        get InvoiceLinePriceRouteTag, "/finance-invoices/line-price", handlers::drafts::line_price, authorize(FinanceInvoicesView, []);
        get DraftInvoiceCreateGetRouteTag, "/finance-invoices/create", handlers::drafts::create_get, modal, authorize(FinanceInvoicesMutate, []);
        post DraftInvoiceCreatePostRouteTag, "/finance-invoices/create", handlers::drafts::create_post, authorize(FinanceInvoicesMutate, []);
        get DraftInvoiceDetailRouteTag, "/finance-invoices/i/{id}", handlers::drafts::detail, authorize(FinanceInvoicesView, []);
        get DraftInvoiceEditGetRouteTag, "/finance-invoices/i/{id}/edit", handlers::drafts::edit_get, modal, authorize(FinanceInvoicesMutate, []);
        post DraftInvoiceEditPostRouteTag, "/finance-invoices/i/{id}/edit", handlers::drafts::edit_post, authorize(FinanceInvoicesMutate, []);
        get DraftInvoiceDeleteGetRouteTag, "/finance-invoices/i/{id}/delete", handlers::drafts::delete_get, modal, authorize(FinanceInvoicesMutate, []);
        post DraftInvoiceDeletePostRouteTag, "/finance-invoices/i/{id}/delete", bare handlers::drafts::delete_post, fragment(DraftInvoiceDeleteModalKey), authorize(FinanceInvoicesMutate, []);
        post DraftInvoicePostRouteTag, "/finance-invoices/i/{id}/post", bare handlers::drafts::post_invoice, redirect, authorize(FinanceInvoicesMutate, []);
        get DraftInvoiceBulkDeleteGetRouteTag, "/finance-invoices/bulk-delete", handlers::drafts::bulk_delete_get, modal, authorize(FinanceInvoicesMutate, []);
        post DraftInvoiceBulkDeletePostRouteTag, "/finance-invoices/bulk-delete", bare handlers::drafts::bulk_delete_post, fragment(DraftInvoiceBulkDeleteModalKey), authorize(FinanceInvoicesMutate, []);
        get DraftInvoiceBulkEditGetRouteTag, "/finance-invoices/bulk-edit", handlers::drafts::bulk_edit_get, modal, authorize(FinanceInvoicesMutate, []);
        post DraftInvoiceBulkEditPostRouteTag, "/finance-invoices/bulk-edit", handlers::drafts::bulk_edit_post, authorize(FinanceInvoicesMutate, []);
        post DraftInvoiceBulkPostRouteTag, "/finance-invoices/bulk-post", bare handlers::drafts::bulk_post, redirect, authorize(FinanceInvoicesMutate, []);
        get DraftInvoicePdfModalRouteTag, "/finance-invoices/i/{id}/pdf", bare handlers::pdf::draft_pdf_modal, modal, authorize(FinanceInvoicesView, []);
        get DraftInvoicePdfRouteTag, "/finance-invoices/i/{id}/pdf/file", bare handlers::pdf::draft_pdf, file, authorize(FinanceInvoicesView, []);
        get DraftInvoiceMultiSelectRouteTag, "/finance-invoices/pick", handlers::drafts::multi_select, multi_select(DraftInvoiceSelectTableKey, DraftInvoiceSelectModalKey), authorize(FinanceInvoicesView, []);

        get PostedInvoiceDetailRouteTag, "/finance-invoices/posted/{id}", handlers::posted::detail, authorize(FinanceInvoicesView, []);
        get PostedInvoiceCancelGetRouteTag, "/finance-invoices/posted/{id}/cancel", handlers::posted::cancel_get, authorize(FinanceInvoicesMutate, []);
        post PostedInvoiceCancelRouteTag, "/finance-invoices/posted/{id}/cancel", bare handlers::posted::cancel_invoice, redirect, authorize(FinanceInvoicesMutate, []);
        get PostedInvoiceBulkCancelGetRouteTag, "/finance-invoices/bulk-cancel", handlers::posted::bulk_cancel_get, authorize(FinanceInvoicesMutate, []);
        post PostedInvoiceBulkCancelPostRouteTag, "/finance-invoices/bulk-cancel", bare handlers::posted::bulk_cancel_post, redirect, authorize(FinanceInvoicesMutate, []);
        get PostedInvoicePdfModalRouteTag, "/finance-invoices/posted/{id}/pdf", bare handlers::pdf::posted_pdf_modal, modal, authorize(FinanceInvoicesView, []);
        get PostedInvoicePdfRouteTag, "/finance-invoices/posted/{id}/pdf/file", bare handlers::pdf::posted_pdf, file, authorize(FinanceInvoicesView, []);
        get PostedInvoiceFkSelectRouteTag, "/finance-invoices/posted/pick", handlers::payments::posted_fk_select, fk_select(PostedInvoiceSelectTableKey, PostedInvoiceSelectModalKey), authorize(FinanceInvoicesView, []);

        get CancelledInvoiceDetailRouteTag, "/finance-invoices/cancelled/{id}", handlers::cancelled::detail, authorize(FinanceInvoicesView, []);
        post CancelledInvoiceNewDraftRouteTag, "/finance-invoices/cancelled/{id}/new-draft", bare handlers::cancelled::new_draft, redirect, authorize(FinanceInvoicesMutate, []);
        post CancelledInvoiceBulkNewDraftRouteTag, "/finance-invoices/cancelled/bulk-new-draft", bare handlers::cancelled::bulk_new_draft, redirect, authorize(FinanceInvoicesMutate, []);
        get CancelledInvoicePdfModalRouteTag, "/finance-invoices/cancelled/{id}/pdf", bare handlers::pdf::cancelled_pdf_modal, modal, authorize(FinanceInvoicesView, []);
        get CancelledInvoicePdfRouteTag, "/finance-invoices/cancelled/{id}/pdf/file", bare handlers::pdf::cancelled_pdf, file, authorize(FinanceInvoicesView, []);

        get PaidInvoiceDetailRouteTag, "/finance-invoices/paid/{id}", handlers::settlements::paid_detail, authorize(FinanceInvoicesView, []);
        get PartiallyPaidInvoiceDetailRouteTag, "/finance-invoices/partial/{id}", handlers::settlements::partial_detail, authorize(FinanceInvoicesView, []);

        get PaymentListRouteTag, "/finance-invoices/payments", handlers::payments::list, fragment(PaymentTableKey), authorize(FinanceInvoicesView, []);
        get PaymentCreateGetRouteTag, "/finance-invoices/payments/create", handlers::payments::create_get, modal, authorize(FinanceInvoicesMutate, []);
        post PaymentCreatePostRouteTag, "/finance-invoices/payments/create", handlers::payments::create_post, authorize(FinanceInvoicesMutate, []);
        get PaymentDetailRouteTag, "/finance-invoices/payments/{id}", handlers::payments::detail, authorize(FinanceInvoicesView, []);

        get PaymentBatchCreateGetRouteTag, "/finance-invoices/payments/batch/create", handlers::payment_batches::create_get, modal, authorize(FinanceInvoicesMutate, []);
        post PaymentBatchCreatePostRouteTag, "/finance-invoices/payments/batch/create", handlers::payment_batches::create_post, authorize(FinanceInvoicesMutate, []);
        get PaymentBatchDetailRouteTag, "/finance-invoices/payment-batches/{id}", handlers::payment_batches::detail, authorize(FinanceInvoicesView, []);

        get InvoicePreferencesRouteTag, "/finance-invoices/preferences", handlers::preferences::invoice_preferences_get, authorize(FinanceInvoicesView, []);
        post InvoicePreferencesPostRouteTag, "/finance-invoices/preferences", bare handlers::preferences::invoice_preferences_post, redirect, authorize(FinanceInvoicesMutate, []);
        get PaymentPreferencesRouteTag, "/finance-invoices/payment-preferences", handlers::preferences::payment_preferences_get, authorize(FinanceInvoicesView, []);
        post PaymentPreferencesPostRouteTag, "/finance-invoices/payment-preferences", bare handlers::preferences::payment_preferences_post, redirect, authorize(FinanceInvoicesMutate, []);

        get PaidInvoicePdfModalRouteTag, "/finance-invoices/paid/{id}/pdf", bare handlers::pdf::paid_pdf_modal, modal, authorize(FinanceInvoicesView, []);
        get PaidInvoicePdfRouteTag, "/finance-invoices/paid/{id}/pdf/file", bare handlers::pdf::paid_pdf, file, authorize(FinanceInvoicesView, []);
        get PartiallyPaidInvoicePdfModalRouteTag, "/finance-invoices/partial/{id}/pdf", bare handlers::pdf::partially_paid_pdf_modal, modal, authorize(FinanceInvoicesView, []);
        get PartiallyPaidInvoicePdfRouteTag, "/finance-invoices/partial/{id}/pdf/file", bare handlers::pdf::partially_paid_pdf, file, authorize(FinanceInvoicesView, []);
        get InvoiceBulkPdfsRouteTag, "/finance-invoices/bulk-pdfs", bare handlers::pdf::bulk_pdfs, file, authorize(FinanceInvoicesView, []);

        post InvoicePdfPreviewPostRouteTag, "/finance-invoices/invoice-pdf-preview", bare handlers::invoice_pdf_preview::modal_post, modal, authorize(FinanceInvoicesMutate, []);
        get InvoicePdfPreviewPdfRouteTag, "/finance-invoices/invoice-pdf-preview/{token}", bare handlers::invoice_pdf_preview::pdf_get, file, param token: String, authorize(FinanceInvoicesView, []);
    ]
}
