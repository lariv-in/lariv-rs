//! User administration and authentication for Lariv.
//!
//! Manages users, roles, password hashes, session authentication, and route authorization.
//!
//! # Configurations
//!
//! - `[users]` → [`config::UsersConfig`]: signing key, JWT issuer, initial admin email/password,
//!   and staff roles for user-management routes.
//!
//! # Database models
//!
//! - [`entities::User`]: system users (password hash, email, phone, role reference).
//! - [`entities::Role`]: access control roles (unassigned, superuser, admin, …).
//!
//! # Global layers and middleware
//!
//! - [`layers::AuthLayer`]: validates `auth-token` session cookies; injects authenticated user into context.
//! - [`layers::RoleLayer`]: restricts downstream views by role membership.
//! - [`middleware::RequireAuth`], [`middleware::RequireStaff`]: Axum extractors wrapping the same logic.
//!
//! # Templates
//!
//! Login, signup, logout, user/role CRUD, self-profile, and change-password pages (see [`templates`]).
//!
//! # Routes
//!
//! - `/users/login`, `/users/signup`, `/users/logout`, `/users/unauthenticated`, `/users/success`
//! - `/users/self`, `/users/self/edit`, `/users/self/change-password`
//! - `/users`, `/users/create`, `/users/u/{id}`, edit/delete/change-password variants
//! - `/users/roles`, `/users/roles/create`, `/users/roles/{id}`, edit/delete variants
//!
//! # CLI commands
//!
//! - `createsuperuser` — manually create a superuser account
//! - `changepassword` — change a user's password by email
//! - `revalidate_users` — normalize user email and phone formats

pub mod apps;
pub mod auth;
pub mod cli;
pub mod config;
pub mod entities;
pub mod error;
pub mod forms;
pub mod handlers;
pub mod jwt;
pub mod keys;
pub mod create_modals;
pub mod layers;
pub mod middleware;
pub mod migrations;
pub mod password;
pub mod routes;
pub mod seed;
pub mod session;
pub mod state;
pub mod templates;

#[cfg(test)]
mod tests;

use frunk::{HCons, HNil, hlist::HList};

use crate::plugin_install::define_plugin_install;
use crate::{
    app::{App, MountedApp},
    capability::{CapStore, define_passthrough_cap},
    config::{ConfigCap, ConfigTag},
    db::{DbCap, DbTag},
    hooks::{AttachState, RunSeed},
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::{GetByCapTag, GetByTag},
    },
};

use config::{UsersConfig, UsersConfigTag};
use state::UsersState;

/// Capability tag for the users plugin state.
pub struct UsersTag;

define_passthrough_cap!(UsersStateCap, UsersTag, UsersState);

define_plugin_install! {
    plugin: UsersTag;
    /// Register users deferred hooks and config section.
    steps: [
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        config(UsersConfigTag, UsersConfig),
        http(routes::Hook),
        state(StateHook),
        seeds(SeedsHook),
        commands(cli::Hook),
    ]
}

/// Attaches [`UsersState`] (DB connection + resolved signing keys) at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, CfgIdx, Configs, UsersCfgIdx, TagProof>
    AttachState<L, (DbIdx, CfgIdx, Configs, UsersCfgIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: GetByCapTag<ConfigTag, CfgIdx, Value = ConfigCap<HNil, Configs>>,
    Configs: GetByTag<UsersConfigTag, UsersCfgIdx, Value = UsersConfig>,
    L: HList + CapTagAbsent<UsersTag, TagProof>,
{
    type Output = HCons<UsersStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        let config = <Configs as GetByTag<UsersConfigTag, UsersCfgIdx>>::get_by_tag(
            &app.get_capability::<ConfigTag, CfgIdx>().items,
        )
        .clone();
        app.add_capability(CapStore::with_items(UsersState::new(conn, config)))
    }
}

/// Seeds default roles and the configured admin user after mount.
#[derive(Clone, Copy, Default)]
pub struct SeedsHook;

#[async_trait::async_trait]
impl<M, UsersIdx> RunSeed<M, UsersIdx> for SeedsHook
where
    M: GetByTag<UsersTag, UsersIdx, Value = UsersState> + Sync,
{
    async fn run_seed(app: &MountedApp<M>) -> anyhow::Result<()> {
        seed::seed(app.get_capability_output::<UsersTag, UsersIdx>()).await?;
        Ok(())
    }
}
