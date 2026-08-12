use super::{handlers, keys::CreditNoteTableKey};

crate::define_plugin_routes! {
    plugin: FinanceCreditnotesTag;
    routes: [
        get CreditNoteDefaultRouteTag, "/finance-credit-notes", handlers::credit_notes::list, fragment(CreditNoteTableKey);
        get CreditNoteDetailRouteTag, "/finance-credit-notes/c/{id}", handlers::credit_notes::detail;
    ]
}
