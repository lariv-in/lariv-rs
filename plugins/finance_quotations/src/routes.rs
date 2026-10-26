use super::{
    handlers,
    keys::{QuotationDeleteModalKey, QuotationTableKey},
};

pub struct FinanceQuotationsView;

pub struct FinanceQuotationsMutate;

lariv_core::define_plugin_routes! {
    plugin: FinanceQuotationsTag;
    prefix: "/dashboard";
    routes: [
        get QuotationDefaultRouteTag, "/finance-quotations", handlers::quotations::list, fragment(QuotationTableKey), authorize(FinanceQuotationsView, []);
        get QuotationGoodsLinePriceRouteTag, "/finance-quotations/goods-line-price", handlers::quotations::goods_line_price, authorize(FinanceQuotationsView, []);
        get QuotationServiceLinePriceRouteTag, "/finance-quotations/service-line-price", handlers::quotations::service_line_price, authorize(FinanceQuotationsView, []);
        get QuotationCreateGetRouteTag, "/finance-quotations/create", handlers::quotations::create_get, modal, authorize(FinanceQuotationsMutate, []);
        post QuotationCreatePostRouteTag, "/finance-quotations/create", handlers::quotations::create_post, authorize(FinanceQuotationsMutate, []);
        get QuotationDetailRouteTag, "/finance-quotations/q/{id}", handlers::quotations::detail, authorize(FinanceQuotationsView, []);
        get QuotationEditGetRouteTag, "/finance-quotations/q/{id}/edit", handlers::quotations::edit_get, modal, authorize(FinanceQuotationsMutate, []);
        post QuotationEditPostRouteTag, "/finance-quotations/q/{id}/edit", handlers::quotations::edit_post, authorize(FinanceQuotationsMutate, []);
        get QuotationDeleteGetRouteTag, "/finance-quotations/q/{id}/delete", handlers::quotations::delete_get, modal, authorize(FinanceQuotationsMutate, []);
        post QuotationDeletePostRouteTag, "/finance-quotations/q/{id}/delete", bare handlers::quotations::delete_post, fragment(QuotationDeleteModalKey), authorize(FinanceQuotationsMutate, []);
        post QuotationCopyPostRouteTag, "/finance-quotations/q/{id}/copy", bare handlers::quotations::copy_post, redirect, authorize(FinanceQuotationsMutate, []);
        get QuotationPdfModalRouteTag, "/finance-quotations/q/{id}/pdf", bare handlers::pdf::pdf_modal, modal, authorize(FinanceQuotationsView, []);
        get QuotationPdfRouteTag, "/finance-quotations/q/{id}/pdf/file", bare handlers::pdf::pdf_file, file, authorize(FinanceQuotationsView, []);
        get QuotationMailRouteTag, "/finance-quotations/q/{id}/mail", bare handlers::pdf::mail_file, file, authorize(FinanceQuotationsView, []);
        get QuotationCreateInvoiceGetRouteTag, "/finance-quotations/q/{id}/create-invoice", handlers::quotations::create_invoice_get, modal, authorize(FinanceQuotationsMutate, []);
        post QuotationCreateInvoicePostRouteTag, "/finance-quotations/q/{id}/create-invoice", bare handlers::quotations::create_invoice_post, redirect, authorize(FinanceQuotationsMutate, []);
    ]
}
