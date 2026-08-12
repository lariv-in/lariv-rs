use super::{
    handlers,
    keys::{
        InvoiceHubTableKey, PaymentTableKey,
        PostedInvoiceSelectModalKey, PostedInvoiceSelectTableKey,
    },
};

crate::define_plugin_routes! {
    plugin: FinanceInvoicesTag;
    routes: [
        get InvoiceDefaultRouteTag, "/finance-invoices", handlers::hub::hub, fragment(InvoiceHubTableKey);
        get DraftInvoiceCreateGetRouteTag, "/finance-invoices/create", handlers::drafts::create_get, modal;
        post DraftInvoiceCreatePostRouteTag, "/finance-invoices/create", handlers::drafts::create_post;
        get DraftInvoiceDetailRouteTag, "/finance-invoices/i/{id}", handlers::drafts::detail;
        get DraftInvoiceEditGetRouteTag, "/finance-invoices/i/{id}/edit", handlers::drafts::edit_get;
        post DraftInvoiceEditPostRouteTag, "/finance-invoices/i/{id}/edit", bare handlers::drafts::edit_post, redirect;
        post DraftInvoiceDeletePostRouteTag, "/finance-invoices/i/{id}/delete", bare handlers::drafts::delete_post, redirect;
        post DraftInvoicePostRouteTag, "/finance-invoices/i/{id}/post", bare handlers::drafts::post_invoice, redirect;
        get DraftInvoicePdfRouteTag, "/finance-invoices/i/{id}/pdf/", bare handlers::pdf::draft_pdf, file;

        get PostedInvoiceDetailRouteTag, "/finance-invoices/posted/{id}", handlers::posted::detail;
        get PostedInvoiceCancelGetRouteTag, "/finance-invoices/posted/{id}/cancel", handlers::posted::cancel_get;
        post PostedInvoiceCancelRouteTag, "/finance-invoices/posted/{id}/cancel", bare handlers::posted::cancel_invoice, redirect;
        get PostedInvoicePdfRouteTag, "/finance-invoices/posted/{id}/pdf/", bare handlers::pdf::posted_pdf, file;
        get PostedInvoiceFkSelectRouteTag, "/finance-invoices/posted/pick", handlers::payments::posted_fk_select, fk_select(PostedInvoiceSelectTableKey, PostedInvoiceSelectModalKey);

        get CancelledInvoiceDetailRouteTag, "/finance-invoices/cancelled/{id}", handlers::cancelled::detail;
        post CancelledInvoiceNewDraftRouteTag, "/finance-invoices/cancelled/{id}/new-draft", bare handlers::cancelled::new_draft, redirect;
        get CancelledInvoicePdfRouteTag, "/finance-invoices/cancelled/{id}/pdf/", bare handlers::pdf::cancelled_pdf, file;

        get PaidInvoiceDetailRouteTag, "/finance-invoices/paid/{id}", handlers::settlements::paid_detail;
        get PartiallyPaidInvoiceDetailRouteTag, "/finance-invoices/partial/{id}", handlers::settlements::partial_detail;

        get PaymentListRouteTag, "/finance-invoices/payments", handlers::payments::list, fragment(PaymentTableKey);
        get PaymentCreateGetRouteTag, "/finance-invoices/payments/create", handlers::payments::create_get, modal;
        post PaymentCreatePostRouteTag, "/finance-invoices/payments/create", handlers::payments::create_post;
        get PaymentDetailRouteTag, "/finance-invoices/payments/{id}", handlers::payments::detail;

        get PaymentBatchCreateGetRouteTag, "/finance-invoices/payments/batch/create", handlers::payment_batches::create_get, modal;
        post PaymentBatchCreatePostRouteTag, "/finance-invoices/payments/batch/create", handlers::payment_batches::create_post;
        get PaymentBatchDetailRouteTag, "/finance-invoices/payment-batches/{id}", handlers::payment_batches::detail;

        get InvoicePreferencesRouteTag, "/finance-invoices/preferences", handlers::preferences::invoice_preferences_get;
        post InvoicePreferencesPostRouteTag, "/finance-invoices/preferences", bare handlers::preferences::invoice_preferences_post, redirect;
        get PaymentPreferencesRouteTag, "/finance-invoices/payment-preferences", handlers::preferences::payment_preferences_get;
        post PaymentPreferencesPostRouteTag, "/finance-invoices/payment-preferences", bare handlers::preferences::payment_preferences_post, redirect;

        get PaidInvoicePdfRouteTag, "/finance-invoices/paid/{id}/pdf/", bare handlers::pdf::paid_pdf, file;
        get PartiallyPaidInvoicePdfRouteTag, "/finance-invoices/partial/{id}/pdf/", bare handlers::pdf::partially_paid_pdf, file;

        post InvoicePdfPreviewPostRouteTag, "/finance-invoices/invoice-pdf-preview", bare handlers::invoice_pdf_preview::modal_post, modal;
        get InvoicePdfPreviewPdfRouteTag, "/finance-invoices/invoice-pdf-preview/{token}", bare handlers::invoice_pdf_preview::pdf_get, file, param token: String;
    ]
}
