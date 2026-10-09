#![recursion_limit = "512"]

//! Lariv application kernel and bundled plugins.
//!
//! This crate re-exports [`lariv_core`] and each plugin behind the same feature flags
//! and module paths as before the workspace split.
extern crate self as lariv_rs;

#[cfg(feature = "cap-llm")]
pub use lariv_core::llm_tools;
#[cfg(not(feature = "cap-llm"))]
pub use lariv_core::llm_tools;
#[cfg(feature = "cap-llm")]
pub use lariv_core::rune_env;
#[cfg(not(feature = "cap-llm"))]
pub use lariv_core::rune_env;
#[cfg(feature = "typst")]
pub use lariv_core::typst;
pub use lariv_core::{
    app, apps, auth_hooks, capability, command, components, config, datetime, db, docs, duration,
    export, filestore, genai, grapesjs, hooks, html_form, http, layers, length, migration, picker,
    plugin_install, plugin_routes, role_registry, rt, tag, template, traits, views, web,
};
pub use lariv_core::{
    define_passthrough_cap, define_plugin_install, define_register_apps, define_register_export,
    define_register_items, define_register_migrations, define_replace_templates, impl_create_modal,
    impl_picker_modal, swap_key,
};
pub use lariv_rs_macros::{define_plugin_routes, main};
pub use paste;

pub mod plugins {
    #[cfg(feature = "plugin-blog")]
    pub use lariv_plugin_blog as blog;
    #[cfg(feature = "plugin-contacts")]
    pub use lariv_plugin_contacts as contacts;
    #[cfg(feature = "plugin-crm")]
    pub use lariv_plugin_crm as crm;
    #[cfg(feature = "plugin-customer")]
    pub use lariv_plugin_customer as customer;
    #[cfg(feature = "plugin-dashboard")]
    pub use lariv_plugin_dashboard as dashboard;
    #[cfg(feature = "plugin-documents")]
    pub use lariv_plugin_documents as documents;
    #[cfg(feature = "plugin-export")]
    pub use lariv_plugin_export as export;
    #[cfg(feature = "plugin-filesystem")]
    pub use lariv_plugin_filesystem as filesystem;
    #[cfg(feature = "plugin-finance-accounts")]
    pub use lariv_plugin_finance_accounts as finance_accounts;
    #[cfg(feature = "finance-common")]
    pub use lariv_plugin_finance_common as finance_common;
    #[cfg(feature = "plugin-finance-creditnotes")]
    pub use lariv_plugin_finance_creditnotes as finance_creditnotes;
    #[cfg(feature = "plugin-finance-indian")]
    pub use lariv_plugin_finance_indian as finance_indian;
    #[cfg(feature = "plugin-finance-invoices")]
    pub use lariv_plugin_finance_invoices as finance_invoices;
    #[cfg(feature = "plugin-finance-products")]
    pub use lariv_plugin_finance_products as finance_products;
    #[cfg(feature = "plugin-finance-taxes")]
    pub use lariv_plugin_finance_taxes as finance_taxes;
    #[cfg(feature = "plugin-forms")]
    pub use lariv_plugin_forms as forms;
    #[cfg(feature = "plugin-hr")]
    pub use lariv_plugin_hr as hr;
    #[cfg(feature = "plugin-import")]
    pub use lariv_plugin_import as import;
    #[cfg(feature = "plugin-inventory")]
    pub use lariv_plugin_inventory as inventory;
    #[cfg(feature = "plugin-llm-assistant")]
    pub use lariv_plugin_llm_assistant as llm_assistant;
    #[cfg(feature = "plugin-meets")]
    pub use lariv_plugin_meets as meets;
    #[cfg(feature = "plugin-otp")]
    pub use lariv_plugin_otp as otp;
    #[cfg(feature = "plugin-pwa")]
    pub use lariv_plugin_pwa as pwa;
    #[cfg(feature = "plugin-signing")]
    pub use lariv_plugin_signing as signing;
    #[cfg(feature = "plugin-signup")]
    pub use lariv_plugin_signup as signup;
    #[cfg(feature = "plugin-tasks")]
    pub use lariv_plugin_tasks as tasks;
    #[cfg(feature = "plugin-users")]
    pub use lariv_plugin_users as users;
    #[cfg(feature = "plugin-website")]
    pub use lariv_plugin_website as website;
}
