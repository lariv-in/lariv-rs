//! GST component preferences and GSTR-1 JSON download.

pub mod apps;
pub mod details;
pub mod export_action;
pub mod forms;
pub mod gstr1;
pub mod handlers;
pub mod invoice_addon;
pub mod invoice_fields;
pub mod invoice_script;
pub mod migrations;
pub mod routes;
pub mod state;
pub mod templates;
pub mod xlsx;

use frunk::{HCons, hlist::HList};

use lariv_core::app::App;
use lariv_core::capability::CapStore;
use lariv_core::db::{DbCap, DbTag};
use lariv_core::hooks::AttachState;
use lariv_core::traits::{
    add::{AddCapability, CapTagAbsent},
    get::GetByCapTag,
};

use state::GstExportState;

pub struct FinanceGstExportTag;

lariv_core::define_passthrough_cap!(GstExportStateCap, FinanceGstExportTag, GstExportState);

lariv_core::define_plugin_install! {
    plugin: FinanceGstExportTag;
    steps: [
        cap_hook(lariv_plugin_import_export::export_actions::ExportPageActionsTag, lariv_plugin_import_export::export_actions::ExportPageActionsCap, export_action::Hook),
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        http(routes::Hook),
        state(StateHook),
        rune_env(invoice_script::RuneHook),
    ]
}

#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<FinanceGstExportTag, TagProof>,
{
    type Output = HCons<GstExportStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        invoice_addon::register();
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(GstExportState::new(conn)))
    }
}
