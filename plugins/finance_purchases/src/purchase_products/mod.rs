//! Purchase product catalog. Same shape as finance products, owned by purchases.

pub mod create_modals;
pub mod entities;
pub mod forms;
pub mod handlers;
#[cfg(feature = "plugin-llm-assistant")]
pub mod hitl;
pub mod journal_template;
pub mod journal_template_editor;
pub mod keys;
pub mod preferences;
pub mod pricing;
pub mod rune_env;
pub mod scope;
pub mod templates;
