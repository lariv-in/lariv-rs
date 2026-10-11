use super::{
    handlers,
    keys::{
        DraftPurchaseBulkDeleteModalKey, DraftPurchaseDeleteModalKey, DraftPurchaseSelectModalKey,
        DraftPurchaseSelectTableKey, PurchaseHubTableKey,
    },
};

pub struct FinancePurchasesView;

pub struct FinancePurchasesMutate;

lariv_core::define_plugin_routes! {
    plugin: FinancePurchasesTag;
    prefix: "/dashboard";
    routes: [
        get PurchaseDefaultRouteTag, "/finance-purchases", handlers::hub::hub, fragment(PurchaseHubTableKey), authorize(FinancePurchasesView, []);
        get PurchaseLinePriceRouteTag, "/finance-purchases/line-price", handlers::drafts::line_price, authorize(FinancePurchasesView, []);
        get DraftPurchaseCreateGetRouteTag, "/finance-purchases/create", handlers::drafts::create_get, modal, authorize(FinancePurchasesMutate, []);
        post DraftPurchaseCreatePostRouteTag, "/finance-purchases/create", handlers::drafts::create_post, authorize(FinancePurchasesMutate, []);
        get DraftPurchaseDetailRouteTag, "/finance-purchases/i/{id}", handlers::drafts::detail, authorize(FinancePurchasesView, []);
        get DraftPurchaseEditGetRouteTag, "/finance-purchases/i/{id}/edit", handlers::drafts::edit_get, modal, authorize(FinancePurchasesMutate, []);
        post DraftPurchaseEditPostRouteTag, "/finance-purchases/i/{id}/edit", handlers::drafts::edit_post, authorize(FinancePurchasesMutate, []);
        get DraftPurchaseDeleteGetRouteTag, "/finance-purchases/i/{id}/delete", handlers::drafts::delete_get, modal, authorize(FinancePurchasesMutate, []);
        post DraftPurchaseDeletePostRouteTag, "/finance-purchases/i/{id}/delete", bare handlers::drafts::delete_post, fragment(DraftPurchaseDeleteModalKey), authorize(FinancePurchasesMutate, []);
        post DraftPurchasePostRouteTag, "/finance-purchases/i/{id}/post", bare handlers::drafts::post_purchase, redirect, authorize(FinancePurchasesMutate, []);
        get DraftPurchaseBulkDeleteGetRouteTag, "/finance-purchases/bulk-delete", handlers::drafts::bulk_delete_get, modal, authorize(FinancePurchasesMutate, []);
        post DraftPurchaseBulkDeletePostRouteTag, "/finance-purchases/bulk-delete", bare handlers::drafts::bulk_delete_post, fragment(DraftPurchaseBulkDeleteModalKey), authorize(FinancePurchasesMutate, []);
        get DraftPurchaseBulkEditGetRouteTag, "/finance-purchases/bulk-edit", handlers::drafts::bulk_edit_get, modal, authorize(FinancePurchasesMutate, []);
        post DraftPurchaseBulkEditPostRouteTag, "/finance-purchases/bulk-edit", handlers::drafts::bulk_edit_post, authorize(FinancePurchasesMutate, []);
        post DraftPurchaseBulkPostRouteTag, "/finance-purchases/bulk-post", bare handlers::drafts::bulk_post, redirect, authorize(FinancePurchasesMutate, []);
        get DraftPurchasePdfModalRouteTag, "/finance-purchases/i/{id}/pdf", bare handlers::pdf::draft_pdf_modal, modal, authorize(FinancePurchasesView, []);
        get DraftPurchasePdfRouteTag, "/finance-purchases/i/{id}/pdf/file", bare handlers::pdf::draft_pdf, file, authorize(FinancePurchasesView, []);
        get DraftPurchaseMultiSelectRouteTag, "/finance-purchases/pick", handlers::drafts::multi_select, multi_select(DraftPurchaseSelectTableKey, DraftPurchaseSelectModalKey), authorize(FinancePurchasesView, []);

        get PostedPurchaseDetailRouteTag, "/finance-purchases/posted/{id}", handlers::posted::detail, authorize(FinancePurchasesView, []);
        get PostedPurchaseCancelGetRouteTag, "/finance-purchases/posted/{id}/cancel", handlers::posted::cancel_get, authorize(FinancePurchasesMutate, []);
        post PostedPurchaseCancelRouteTag, "/finance-purchases/posted/{id}/cancel", bare handlers::posted::cancel_purchase, redirect, authorize(FinancePurchasesMutate, []);
        get PostedPurchaseBulkCancelGetRouteTag, "/finance-purchases/bulk-cancel", handlers::posted::bulk_cancel_get, authorize(FinancePurchasesMutate, []);
        post PostedPurchaseBulkCancelPostRouteTag, "/finance-purchases/bulk-cancel", bare handlers::posted::bulk_cancel_post, redirect, authorize(FinancePurchasesMutate, []);
        get PostedPurchasePdfModalRouteTag, "/finance-purchases/posted/{id}/pdf", bare handlers::pdf::posted_pdf_modal, modal, authorize(FinancePurchasesView, []);
        get PostedPurchasePdfRouteTag, "/finance-purchases/posted/{id}/pdf/file", bare handlers::pdf::posted_pdf, file, authorize(FinancePurchasesView, []);

        get CancelledPurchaseDetailRouteTag, "/finance-purchases/cancelled/{id}", handlers::cancelled::detail, authorize(FinancePurchasesView, []);
        post CancelledPurchaseNewDraftRouteTag, "/finance-purchases/cancelled/{id}/new-draft", bare handlers::cancelled::new_draft, redirect, authorize(FinancePurchasesMutate, []);
        post CancelledPurchaseBulkNewDraftRouteTag, "/finance-purchases/cancelled/bulk-new-draft", bare handlers::cancelled::bulk_new_draft, redirect, authorize(FinancePurchasesMutate, []);
        get CancelledPurchasePdfModalRouteTag, "/finance-purchases/cancelled/{id}/pdf", bare handlers::pdf::cancelled_pdf_modal, modal, authorize(FinancePurchasesView, []);
        get CancelledPurchasePdfRouteTag, "/finance-purchases/cancelled/{id}/pdf/file", bare handlers::pdf::cancelled_pdf, file, authorize(FinancePurchasesView, []);

        get PurchasePreferencesRouteTag, "/finance-purchases/preferences", handlers::preferences::purchase_preferences_get, authorize(FinancePurchasesView, []);
        post PurchasePreferencesPostRouteTag, "/finance-purchases/preferences", bare handlers::preferences::purchase_preferences_post, redirect, authorize(FinancePurchasesMutate, []);

        get PurchaseBulkPdfsRouteTag, "/finance-purchases/bulk-pdfs", bare handlers::pdf::bulk_pdfs, file, authorize(FinancePurchasesView, []);

        post PurchasePdfPreviewPostRouteTag, "/finance-purchases/purchase-pdf-preview", bare handlers::purchase_pdf_preview::modal_post, modal, authorize(FinancePurchasesMutate, []);
        get PurchasePdfPreviewPdfRouteTag, "/finance-purchases/purchase-pdf-preview/{token}", bare handlers::purchase_pdf_preview::pdf_get, file, param token: String, authorize(FinancePurchasesView, []);
    ]
}
