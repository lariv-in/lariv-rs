//! Blog plugin — articles and hierarchical tags.
//!
//! Port of Go `p_blog`: CRUD for blogs and tags at `/blog/…`, dashboard tile
//! “Blog”, auth via [`crate::plugins::users::middleware::RequireAuth`].

pub mod apps;
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
