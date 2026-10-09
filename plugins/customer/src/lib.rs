#![feature(impl_trait_in_assoc_type)]
//! Historical customer migrations.
//!
//! Invoices bill a contact or a company. These migrations stay registered so
//! existing databases still recognize applied versions, and fresh installs
//! still create `customers` before the invoice tables reference it. A later
//! finance migration copies those rows and drops the table.

pub mod migrations;

pub struct CustomerTag;

lariv_core::define_plugin_install! {
    plugin: CustomerTag;
    steps: [
        migrations(migrations::Hook),
    ]
}
