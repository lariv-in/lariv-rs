use std::sync::{Mutex, OnceLock};

use async_trait::async_trait;
use maud::Markup;
use sea_orm::DatabaseConnection;

static ACTIONS: OnceLock<Mutex<Vec<&'static dyn QuotationDetailAction>>> = OnceLock::new();

fn action_list() -> &'static Mutex<Vec<&'static dyn QuotationDetailAction>> {
    ACTIONS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Extra buttons on a quotation detail page, registered by other plugins.
#[async_trait]
pub trait QuotationDetailAction: Send + Sync {
    fn id(&self) -> &'static str;
    async fn render(&self, db: &DatabaseConnection, quotation_id: i64) -> Markup;
}

/// Register a detail action. A second registration with the same id is ignored.
pub fn register_quotation_detail_action(action: &'static dyn QuotationDetailAction) {
    let mut list = action_list().lock().unwrap_or_else(|err| err.into_inner());
    if !list.iter().any(|existing| existing.id() == action.id()) {
        list.push(action);
    }
}

/// Markup for every registered detail action, in registration order.
pub async fn render_quotation_detail_actions(db: &DatabaseConnection, quotation_id: i64) -> Markup {
    let actions = action_list()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .clone();
    let mut out = Markup::default();
    for action in actions {
        let piece = action.render(db, quotation_id).await;
        out = maud::html! { (out) (piece) };
    }
    out
}
