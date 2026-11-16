//! Audit log for one invoice, opened from that invoice's detail sidebar.
//!
//! Registers a "Logs" entry on quotation, draft, posted, and cancelled invoice
//! detail menus and serves the shared audit table locked to that document.
//! State changes copy the source document's header and line-item logs onto the new document.

pub mod handlers;
pub mod inherit;
pub mod menu;
pub mod routes;
pub mod state;
pub mod templates;

use frunk::{HCons, hlist::HList};

use lariv_core::app::App;
use lariv_core::capability::{CapStore, define_passthrough_cap};
use lariv_core::db::{DbCap, DbTag};
use lariv_core::hooks::AttachState;
use lariv_core::plugin_install::define_plugin_install;
use lariv_core::traits::{
    add::{AddCapability, CapTagAbsent},
    get::GetByCapTag,
};

use state::InvoiceLogState;

/// Capability tag for the invoice log plugin state.
pub struct FinanceInvoicesLogTag;

define_passthrough_cap!(InvoiceLogStateCap, FinanceInvoicesLogTag, InvoiceLogState);

define_plugin_install! {
    plugin: FinanceInvoicesLogTag;
    /// Register the per-invoice logs pages, routes, and sidebar link.
    steps: [
        cap_hook(lariv_plugin_finance_invoices::invoice_detail_menu::InvoiceDetailMenuTag, lariv_plugin_finance_invoices::invoice_detail_menu::InvoiceDetailMenuCap, menu::Hook),
        templates(templates::Hook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        http(routes::Hook),
        state(StateHook),
    ]
}

/// Attaches [`InvoiceLogState`] and the log-copy addon at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<FinanceInvoicesLogTag, TagProof>,
{
    type Output = HCons<InvoiceLogStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        inherit::register();
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(InvoiceLogState::new(conn)))
    }
}
