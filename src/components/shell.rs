//! Document shells wrapping DaisyUI / HTMX / Alpine chrome.

mod auth;
mod base;
mod scaffold;
mod simple;
mod topbar;

pub use auth::{ShellAuth, shell_auth};
pub use base::{ShellBase, shell_base};
pub use scaffold::{ShellScaffold, shell_scaffold};
pub use simple::{ShellSimple, shell_simple};
pub use topbar::{ShellTopbar, shell_topbar};
