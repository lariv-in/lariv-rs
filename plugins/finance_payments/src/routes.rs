use super::{customer_journal, handlers, keys::PAYMENT_TABLE};

pub struct FinancePaymentsView;

pub struct FinancePaymentsMutate;

lariv_core::define_plugin_routes! {
    plugin: FinancePaymentsTag;
    prefix: "/dashboard";
    routes: [
        get PaymentListRouteTag, "/finance-payments/payments", handlers::payments::list, fragment(PAYMENT_TABLE), authorize(FinancePaymentsView, []);
        get CustomerJournalPaymentsRouteTag, "/finance/customers/{id}/journal/payments", customer_journal::payments, authorize(FinancePaymentsView, []);
        get PaymentCreateGetRouteTag, "/finance-payments/payments/create", handlers::payments::create_get, modal, authorize(FinancePaymentsMutate, []);
        post PaymentCreatePostRouteTag, "/finance-payments/payments/create", handlers::payments::create_post, authorize(FinancePaymentsMutate, []);
        post PaymentJournalLinesRouteTag, "/finance-payments/payment-journal-lines", bare handlers::payments::fill_journal_lines, raw, authorize(FinancePaymentsMutate, []);
        get PaymentDetailRouteTag, "/finance-payments/payments/{id}", handlers::payments::detail, authorize(FinancePaymentsView, []);
    ]
}
