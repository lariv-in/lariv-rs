//! Beginner guides and tutorials for building Lariv applications and plugins.
//!
//! These guides explain concepts step by step with copy-paste examples. They mirror the
//! structure of the API reference on [`crate::app`], [`crate::http`], [`crate::template`],
//! and related modules, but focus on *how to build* rather than *what each type does*.
//!
//! # Start here
//!
//! | Guide | Topic |
//! |-------|-------|
//! | [`quickstart`] | Build a Hello World plugin from scratch |
//! | [`app`] | Plugin entrypoint and `define_plugin_install!` |
//! | [`routes`] | HTTP routing with `define_plugin_routes!` |
//! | [`templates`] | Maud page types and template registration |
//! | [`handlers`] | Axum handlers, auth extractors, HTMX responses |
//! | [`layers`] | View middleware stacks (load, list, create, update, delete) |
//! | [`patch`] | Query and form patchers for list/detail layers |
//! | [`components`] | UI builders (fields, inputs, tables, shells) |
//! | [`config`] | TOML configuration sections |
//! | [`entities`] | SeaORM models and relations |
//! | [`migrations`] | Database schema migrations |
//! | [`commands`] | CLI subcommands |
//!
//! # Project layout
//!
//! A typical Lariv application looks like this:
//!
//! ```text
//! <project root>/
//! ├── Cargo.toml
//! ├── config.toml              # database URL, bind address, plugin sections
//! ├── src/
//! │   ├── main.rs              # or src/bin/lariv.rs — install plugins, serve
//! │   └── plugins/
//! │       └── hello/
//! │           ├── mod.rs       # plugin tag + define_plugin_install!
//! │           ├── routes.rs    # define_plugin_routes!
//! │           ├── templates.rs # RenderTemplate pages + define_register_items!
//! │           ├── handlers.rs  # async Axum handler fns
//! │           ├── config.rs    # optional [hello] TOML section
//! │           ├── entities/    # optional SeaORM models
//! │           ├── migrations/  # optional SeaORM migrators
//! │           └── cli.rs       # optional CLI subcommands
//! └── data/                    # SQLite file (when using default database_url)
//! ```
//!
//! # Application bootstrap
//!
//! Every binary follows the same lifecycle:
//!
//! ```ignore
//! use lariv_rs::app::App;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let app = App::new_web_app();
//!     let app = my_plugin::install(app);
//!     // … install other plugins …
//!     let app = app.load_config("config.toml").await?;
//!     let mounted = app.mount();
//!     mounted.run().await?   // default: serve HTTP
//! }
//! ```
//!
//! CLI subcommands are registered automatically: `migrate`, `seed`, and `serve`.
//!
//! ```text
//! cargo run -- migrate    # apply SeaORM migrations
//! cargo run -- seed       # run startup seed hooks
//! cargo run -- serve      # start the HTTP server (also the default)
//! ```
//!
//! # Plugin file roles
//!
//! | File | Purpose |
//! |------|---------|
//! | `mod.rs` | Plugin tag, `define_plugin_install!`, state hooks |
//! | `routes.rs` | URL paths → handlers via `define_plugin_routes!` |
//! | `templates.rs` | Page structs implementing [`RenderTemplate`](crate::template::RenderTemplate) |
//! | `handlers.rs` | Async functions called by routes; build pages and return HTML |
//! | `config.rs` | Struct + [`ConfigSection`](crate::config::ConfigSection) for TOML |
//! | `entities/` | SeaORM `Model` / `ActiveModel` definitions |
//! | `migrations/` | SeaORM migration modules |
//! | `cli.rs` | Clap subcommands via [`CommandRegistrar`](crate::command::CommandRegistrar) |
//! | `apps.rs` | Dashboard tile via [`define_register_apps!`](crate::apps::define_register_apps) |
//!
//! See individual guides for worked examples of each file.

pub mod app;
pub mod commands;
pub mod components;
pub mod config;
pub mod entities;
pub mod handlers;
pub mod layers;
pub mod migrations;
pub mod patch;
pub mod quickstart;
pub mod routes;
pub mod templates;
