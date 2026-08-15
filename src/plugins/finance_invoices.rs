//! Finance invoices plugin.

pub mod accounting_preferences_patch;
pub mod accounting_sidebar;
pub mod apps;
pub mod components;
pub mod create_modals;
pub mod draft_form_addon;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod invoice_pdf_addon;
pub mod invoice_pdf_assets;
pub mod invoice_pdf_template;
pub mod keys;
pub mod logic;
pub mod migrations;
pub mod payment_term_kind;
pub mod preferences_hints;
pub mod routes;
pub mod scope;
pub mod source_docs;
pub mod state;
pub mod templates;

pub use payment_term_kind::{PaymentTermAmountKind, PaymentTermDateKind};
pub use invoice_pdf_assets::VnodeImageContext;
pub use invoice_pdf_template::DEFAULT_INVOICE_PDF_TEMPLATE;

use frunk::{HCons, hlist::HList};

use crate::{
    app::App,
    capability::CapStore,
    db::{DbCap, DbTag},
    hooks::AttachState,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::GetByCapTag,
    },
};

use state::InvoicesState;

pub struct FinanceInvoicesTag;

crate::define_passthrough_cap!(FinanceInvoicesStateCap, FinanceInvoicesTag, InvoicesState);

crate::define_plugin_install! {
    plugin: FinanceInvoicesTag;
    steps: [
        cap_hook(crate::plugins::finance_accounts::accounting_sidebar::AccountingSidebarTag, crate::plugins::finance_accounts::accounting_sidebar::AccountingSidebarCap, accounting_sidebar::Hook),
        cap_hook(crate::plugins::finance_accounts::SourceDocTag, crate::plugins::finance_accounts::SourceDocCap, source_docs::Hook),
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        http(routes::Hook),
        state(StateHook),
    ]
}

#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<FinanceInvoicesTag, TagProof>,
{
    type Output = HCons<FinanceInvoicesStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(InvoicesState::new(conn)))
    }
}
