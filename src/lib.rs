#![feature(impl_trait_in_assoc_type)]
#![recursion_limit = "512"]

//! Proc-macro derives expand to `::lariv_rs::…` paths; alias this crate for in-tree use.
extern crate self as lariv_rs;

pub mod app;
pub mod apps;
pub mod capability;
pub mod command;
pub mod components;
pub mod config;
pub mod db;
pub mod export;
pub mod genai;
pub mod grapesjs;
pub mod hooks;
pub mod html_form;
pub mod http;
pub mod layers;
pub mod llm_tools;
pub mod rune_env;
pub mod migration;
pub mod plugin_install;
pub mod plugin_routes;
pub mod plugins;
pub mod tag;
pub mod template;
pub mod traits;
pub mod views;
pub mod web;

pub use lariv_rs_macros::define_plugin_routes;
