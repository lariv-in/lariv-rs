//! OTP plugin — password recovery via SMS (MSG91) and email (SMTP).
//!
//! Port of Go `p_otp`.

pub mod adapters;
pub mod apps;
pub mod entities;
pub mod error;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod migrations;
pub mod otp;
pub mod preferences;
pub mod routes;
pub mod state;
pub mod templates;

use frunk::{HCons, hlist::HList};

use crate::plugin_install::define_plugin_install;
use crate::{
    app::App,
    capability::{CapStore, define_passthrough_cap},
    db::{DbCap, DbTag},
    hooks::AttachState,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::GetByCapTag,
    },
};

use state::OtpState;

/// Capability tag for the OTP plugin state.
pub struct OtpTag;

define_passthrough_cap!(OtpStateCap, OtpTag, OtpState);

define_plugin_install! {
    plugin: OtpTag;
    /// Register OTP deferred hooks (apps, migrations, templates, slots, routes, state).
    steps: [
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook, LoginIdx),
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
    L: HList + CapTagAbsent<OtpTag, TagProof>,
{
    type Output = HCons<OtpStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(OtpState::new(conn)))
    }
}
