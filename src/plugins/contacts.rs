//! Contacts plugin — people and companies.

pub mod apps;
pub mod create_modals;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod routes;
pub mod scope;
pub mod state;
pub mod templates;

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

use state::ContactsState;

pub struct ContactsTag;

crate::define_passthrough_cap!(ContactsStateCap, ContactsTag, ContactsState);

crate::define_plugin_install! {
    plugin: ContactsTag;
    steps: [
        templates(templates::Hook),
        slots(templates::SlotsHook),
        http(routes::Hook),
        state(StateHook),
        apps(apps::Hook),
    ]
}

#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<ContactsTag, TagProof>,
{
    type Output = HCons<ContactsStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(ContactsState::new(conn)))
    }
}
