use super::{handlers, keys::CreditNoteTableKey};

pub struct FinanceCreditNotesView;

crate::define_plugin_routes! {
    plugin: FinanceCreditnotesTag;
    prefix: "/dashboard";
    routes: [
        get CreditNoteDefaultRouteTag, "/finance-credit-notes", handlers::credit_notes::list, fragment(CreditNoteTableKey), authorize(FinanceCreditNotesView, []);
        get CreditNoteDetailRouteTag, "/finance-credit-notes/c/{id}", handlers::credit_notes::detail, authorize(FinanceCreditNotesView, []);
    ]
}
