//! Maud page templates for finance invoices (hub, drafts, quotations, posted, settlements).

use lariv_core::components::{SlotCapability, SlotRegistrar};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{TemplateCapability, TemplateOf, TemplateRegistrar};

pub mod cancel;
pub mod cancelled;
pub mod common;
pub mod confirm;
pub mod draft;
pub mod hub;
pub mod pay;
pub mod posted;
pub mod preferences;
pub mod quotation;

pub use cancel::{CancelBulkInvoicePage, CancelInvoicePage};
pub use cancelled::CancelledInvoiceDetailPage;
pub use confirm::{
    ConfirmBulkDeletePage, ConfirmDeletePage, ConfirmInvoiceTransition, ResetToDraftDeleteItem,
    confirm_invoice_transition, posted_reset_to_draft_modal,
};
pub use draft::{
    DraftInvoiceBulkEditModalPage, DraftInvoiceCreateModalPage, DraftInvoiceDetailPage,
    DraftInvoiceEditModalPage, DraftInvoiceSelectPage, DraftInvoiceSelectRow,
};
pub use hub::{InvoiceHubPage, InvoiceRow};
pub use pay::InvoicePayModalPage;
pub use posted::{
    PaidInvoiceDetailPage, PartiallyPaidInvoiceDetailPage, PostedInvoiceDetailPage,
    SettlementDetailContext,
};
pub use preferences::InvoicePreferencesPage;
pub use quotation::{
    QuotationInvoiceBulkEditModalPage, QuotationInvoiceCreateModalPage, QuotationInvoiceDetailPage,
    QuotationInvoiceEditModalPage, QuotationInvoiceSelectPage, QuotationInvoiceSelectRow,
};

lariv_core::define_register_items! {
    plugin: FinanceInvoicesTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        InvoiceHubIdx: InvoiceHubPageTag => InvoiceHubPage,
        DraftInvoiceEditModalIdx: DraftInvoiceEditModalPageTag => DraftInvoiceEditModalPage,
        DraftInvoiceCreateModalIdx: DraftInvoiceCreateModalPageTag => DraftInvoiceCreateModalPage,
        DraftInvoiceBulkEditModalIdx: DraftInvoiceBulkEditModalPageTag => DraftInvoiceBulkEditModalPage,
        DraftInvoiceDetailIdx: DraftInvoiceDetailPageTag => DraftInvoiceDetailPage,
        DraftInvoiceSelectIdx: DraftInvoiceSelectPageTag => DraftInvoiceSelectPage,
        QuotationInvoiceEditModalIdx: QuotationInvoiceEditModalPageTag => QuotationInvoiceEditModalPage,
        QuotationInvoiceCreateModalIdx: QuotationInvoiceCreateModalPageTag => QuotationInvoiceCreateModalPage,
        QuotationInvoiceBulkEditModalIdx: QuotationInvoiceBulkEditModalPageTag => QuotationInvoiceBulkEditModalPage,
        QuotationInvoiceDetailIdx: QuotationInvoiceDetailPageTag => QuotationInvoiceDetailPage,
        QuotationInvoiceSelectIdx: QuotationInvoiceSelectPageTag => QuotationInvoiceSelectPage,
        PostedInvoiceDetailIdx: PostedInvoiceDetailPageTag => PostedInvoiceDetailPage,
        PaidInvoiceDetailIdx: PaidInvoiceDetailPageTag => PaidInvoiceDetailPage,
        PartiallyPaidInvoiceDetailIdx: PartiallyPaidInvoiceDetailPageTag => PartiallyPaidInvoiceDetailPage,
        CancelledInvoiceDetailIdx: CancelledInvoiceDetailPageTag => CancelledInvoiceDetailPage,
        CancelInvoiceIdx: CancelInvoicePageTag => CancelInvoicePage,
        CancelBulkInvoiceIdx: CancelBulkInvoicePageTag => CancelBulkInvoicePage,
        InvoicePayModalIdx: InvoicePayModalPageTag => InvoicePayModalPage,
        InvoicePreferencesIdx: InvoicePreferencesPageTag => InvoicePreferencesPage,
        ConfirmDeleteIdx: DraftInvoiceConfirmDeletePageTag => ConfirmDeletePage,
        ConfirmBulkDeleteIdx: DraftInvoiceConfirmBulkDeletePageTag => ConfirmBulkDeletePage,
    ]
}

lariv_core::define_register_items! {
    plugin: FinanceInvoicesTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}
