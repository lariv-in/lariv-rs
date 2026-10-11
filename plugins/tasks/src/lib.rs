#![feature(impl_trait_in_assoc_type)]
//! Tasks plugin — statuses, assigned work, and activity logs.
//!
//! CRUD for tasks at `/tasks/…` and statuses at `/tasks/statuses/…`.
//! A task may optionally point at a parent task, forming a tree.
//! Auth via [`lariv_plugin_users::middleware::RequireAuth`]. A superuser sees every
//! task. Any other user sees and changes status only for tasks assigned to them.
//! Creating, editing, and deleting tasks stays superuser-only.
//!
//! Web Push uses `[tasks]` (`vapidPublicKey`, `vapidPrivateKey`, `vapidSubject`).
//! Install this plugin after the PWA plugin so `/sw/tasks.js` is imported by
//! `/serviceworker.js`. The periodic worker notifies the assignee when it
//! creates a task.

pub mod apps;
pub mod color;
pub mod config;
pub mod create_modals;
pub mod crumbs;
pub mod detail_menu;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod logic;
pub mod migrations;
pub mod push;
pub mod push_ui;
pub mod routes;
pub mod scope;
pub mod serve_startup;
pub mod state;
pub mod templates;
pub mod worker;

use frunk::{HCons, HNil, hlist::HList};

use lariv_core::app::App;
use lariv_core::capability::CapStore;
use lariv_core::config::{ConfigCap, ConfigTag};
use lariv_core::db::{DbCap, DbTag};
use lariv_core::hooks::AttachState;
use lariv_core::traits::{
    add::{AddCapability, CapTagAbsent},
    get::{GetByCapTag, GetByTag},
};

use config::{TasksConfig, TasksConfigTag};
use push::PushSettings;
use state::TasksState;

/// Capability tag for the tasks plugin.
pub struct TasksTag;

lariv_core::define_passthrough_cap!(TasksStateCap, TasksTag, TasksState);

lariv_core::define_plugin_install! {
    plugin: TasksTag;
    steps: [
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        cap_hook(lariv_plugin_pwa::service_worker::ServiceWorkerTag, lariv_plugin_pwa::service_worker::ServiceWorkerCap, push::ServiceWorkerHook),
        config(TasksConfigTag, TasksConfig),
        http(routes::Hook),
        state(StateHook),
        serve_startup(serve_startup::ServeStartupHook),
    ]
}

/// Attaches [`TasksState`] (DB connection and VAPID settings) at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, CfgIdx, Configs, TasksCfgIdx, TagProof>
    AttachState<L, (DbIdx, CfgIdx, Configs, TasksCfgIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: GetByCapTag<ConfigTag, CfgIdx, Value = ConfigCap<HNil, Configs>>,
    Configs: GetByTag<TasksConfigTag, TasksCfgIdx, Value = TasksConfig>,
    L: HList + CapTagAbsent<TasksTag, TagProof>,
{
    type Output = HCons<TasksStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        let config = <Configs as GetByTag<TasksConfigTag, TasksCfgIdx>>::get_by_tag(
            &app.get_capability::<ConfigTag, CfgIdx>().items,
        )
        .clone();
        let push = PushSettings::from_config(&config);
        app.add_capability(CapStore::with_items(TasksState::new(conn, push)))
    }
}
