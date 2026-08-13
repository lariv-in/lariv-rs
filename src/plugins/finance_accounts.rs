//! Finance accounts plugin (GL hub, sidebar registry, source-doc registry).

pub mod account_select;
pub mod account_validation;
pub mod accounting_detail_menu;
pub mod accounting_preferences_patch;
pub mod accounting_sidebar;
pub mod apps;
pub mod balance_type;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod journal_type;
pub mod keys;
pub mod create_modals;
pub mod logic;
pub mod migrations;
pub mod preferences;
pub mod routes;
pub mod scope;
pub mod source_doc_label;
pub mod source_doc_registry;
pub mod state;
pub mod templates;

pub use account_select::account_select_url_with_balance_type as account_select_route_url;
pub use account_validation::validate_leaf_account_balance_type;
pub use balance_type::BalanceType;
pub use journal_type::JournalType;
pub use source_doc_label::{
    SourceDocDisplay, resolve_source_doc_display, source_doc_ref_summary, source_doc_summary,
    source_doc_type_label,
};
pub use source_doc_registry::{
    SourceDocCap, SourceDocInstance, SourceDocRegistrar, SourceDocRegistry, SourceDocTag,
    SourceDocType, humanize_type_name,
};
pub use state::AccountsState;

pub use apps::ACCOUNTING_APP_KEY;

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

pub struct FinanceAccountsTag;

crate::define_passthrough_cap!(FinanceAccountsStateCap, FinanceAccountsTag, AccountsState);

crate::define_plugin_install! {
    plugin: FinanceAccountsTag;
    steps: [
        cap_attach(accounting_sidebar::AccountingSidebarTag, accounting_sidebar::AccountingSidebarCap, accounting_sidebar::AccountingSidebarCap::<frunk::HNil>::new()),
        cap_hook(accounting_sidebar::AccountingSidebarTag, accounting_sidebar::AccountingSidebarCap, accounting_sidebar::BaseHook),
        cap_attach(source_doc_registry::SourceDocTag, source_doc_registry::SourceDocCap, source_doc_registry::SourceDocCap::<frunk::HNil>::new()),
        cap_hook(source_doc_registry::SourceDocTag, source_doc_registry::SourceDocCap, source_doc_registry::BaseHook),
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
    L: HList + CapTagAbsent<FinanceAccountsTag, TagProof>,
{
    type Output = HCons<FinanceAccountsStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(AccountsState::new(conn)))
    }
}
