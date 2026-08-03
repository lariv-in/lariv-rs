//! CLI subcommands for plugins.
//!
//! # Registering commands (`cli.rs`)
//!
//! Plugins contribute Clap subcommands via [`CommandRegistrar`](crate::command::CommandRegistrar).
//! The users plugin ships `createsuperuser`, `changepassword`, and `revalidate_users` as examples.
//!
//! ```ignore
//! use clap::Parser;
//! use lariv_rs::command::{CommandCapability, CommandRegistrar, RunCommand};
//!
//! #[derive(Parser, Debug)]
//! pub struct GreetArgs {
//!     #[arg(long, default_value = "Developer")]
//!     name: String,
//! }
//!
//! pub struct GreetCommand;
//!
//! #[async_trait::async_trait]
//! impl<M> RunCommand<M> for GreetCommand {
//!     type Args = GreetArgs;
//!
//!     async fn run(args: Self::Args, _app: MountedApp<M>) -> anyhow::Result<()> {
//!         println!("Hello, {}!", args.name);
//!         Ok(())
//!     }
//! }
//!
//! pub struct Hook;
//!
//! impl CommandRegistrar for Hook {
//!     fn register_commands(self, cap: CommandCapability) -> CommandCapability {
//!         cap.register("greet", GreetCommand)
//!     }
//! }
//! ```
//!
//! Add `commands(cli::Hook)` to install steps.
//!
//! # Running commands
//!
//! ```text
//! cargo run -- greet --name Alice
//! cargo run -- migrate
//! cargo run -- seed
//! cargo run -- serve          # default when no subcommand given
//! ```
//!
//! Built-in commands:
//!
//! | Command | Purpose |
//! |---------|---------|
//! | `serve` | Start the HTTP server |
//! | `migrate` | Apply SeaORM migrations |
//! | `seed` | Run startup seed hooks |
//!
//! Plugin commands are merged into the same CLI tree built by [`BuildCli`](crate::command::BuildCli).
