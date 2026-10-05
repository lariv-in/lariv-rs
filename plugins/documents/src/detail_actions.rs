//! Extra buttons on a document detail page, registered by other plugins.

use std::sync::{Mutex, OnceLock};

use async_trait::async_trait;
use maud::Markup;
use sea_orm::DatabaseConnection;

use lariv_plugin_users::state::AuthContext;

static ACTIONS: OnceLock<Mutex<Vec<&'static dyn DocumentDetailAction>>> = OnceLock::new();

fn action_list() -> &'static Mutex<Vec<&'static dyn DocumentDetailAction>> {
    ACTIONS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Context available while rendering document detail actions.
pub struct DocumentDetailActionInput<'a> {
    pub db: &'a DatabaseConnection,
    pub auth: &'a AuthContext,
    pub document_id: i64,
    pub vnode_name: &'a str,
}

/// One plugin's buttons on the document detail page.
#[async_trait]
pub trait DocumentDetailAction: Send + Sync {
    fn id(&self) -> &'static str;

    async fn render(&self, input: &DocumentDetailActionInput<'_>) -> Markup;
}

/// Register a detail action (idempotent by [`DocumentDetailAction::id`]).
pub fn register_document_detail_action(action: &'static dyn DocumentDetailAction) {
    let mut list = action_list().lock().unwrap_or_else(|err| err.into_inner());
    if !list.iter().any(|existing| existing.id() == action.id()) {
        list.push(action);
    }
}

/// HTML for every registered action, in registration order.
pub async fn render_document_actions(input: &DocumentDetailActionInput<'_>) -> String {
    let registered = action_list()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .clone();
    let mut out = String::new();
    for action in registered {
        out.push_str(&action.render(input).await.into_string());
    }
    out
}
