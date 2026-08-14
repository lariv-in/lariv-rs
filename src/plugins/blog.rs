//! Blog post management and hierarchical tagging.
//!
//! CRUD for blog articles and ltree-backed tags at `/blog/…`.
//! Auth via [`crate::plugins::users::middleware::RequireAuth`].
//!
//! # Database models
//!
//! - [`entities::Blog`]: article with title, markdown content, author (`CreatedBy`), and tags.
//! - [`entities::BlogTag`]: hierarchical tags (PostgreSQL `ltree`) with many-to-many blogs.
//!
//! # Routes
//!
//! - `/blog/`, `/blog/create/`, `/blog/{id}/`, edit/delete
//! - `/blog/tags/`, tag CRUD variants

pub mod apps;
pub mod create_modals;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod migrations;
pub mod routes;
pub mod slug;
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

use state::BlogState;

/// Capability tag for the blog plugin state.
pub struct BlogTag;

define_passthrough_cap!(BlogStateCap, BlogTag, BlogState);

define_plugin_install! {
    plugin: BlogTag;
    /// Register blog deferred hooks (apps, migrations, templates, slots, routes, state).
    steps: [
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        http(routes::Hook),
        state(StateHook),
    ]
}

/// Attaches [`BlogState`] (DB connection) at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<BlogTag, TagProof>,
{
    type Output = HCons<BlogStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(BlogState::new(conn)))
    }
}
