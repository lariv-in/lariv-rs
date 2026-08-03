//! One-time password (OTP) delivery and verification.
//!
//! Integrates SMS (MSG91) and SMTP email to send six-digit OTP codes for
//! password recovery and multi-factor auth flows.
//!
//! # Database models
//!
//! - In-memory OTP cache ([`otp::MemoryCache`]) for pending codes.
//! - [`entities::OtpPreferences`]: singleton SMTP and MSG91 configuration.
//!
//! # Templates
//!
//! Forgot-password choice page, SMS/email request forms, verify form, and admin preferences
//! panel (see [`templates`]).
//!
//! # Routes
//!
//! - `/otp/forgot-password/` — SMS vs email recovery choice
//! - `/otp/login/sms/`, `/otp/login/email/` — send OTP codes
//! - `/otp/verify/` — verify code and reset password
//! - `/otp/preferences/` — admin SMTP/gateway settings
//!
//! # Patches applied
//!
//! - `p_users.LoginPage` — inserts "Forgot password?" link on the login page.

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

/// Attaches [`OtpState`] (DB connection) at app mount.
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
