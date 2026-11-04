//! Declarative plugin `install` helper — expands `After*` type aliases, `InstallOutput`, and `install`.
//!
//! Each plugin calls [`define_plugin_install!`] once at its crate root (`plugins/<name>.rs`).
//! Steps run in list order; each prepends a tagged hook onto the matching core capability
//! (or registers config / state hooks). At app mount, hooks fold in reverse registration
//! order (tail first).
//!
//! # DSL
//!
//! ```ignore
//! define_plugin_install! {
//!     plugin: PluginTag;          // capability tag (hook identity + state key)
//!     /// Rustdoc for `install`   // optional; attached to generated `install`
//!     steps: [ step, step, ... ];   // ordered install chain
//!     finish: add_capability(CapTy, expr);  // optional eager capability (no state hook)
//! }
//! ```
//!
//! ## Step kinds
//!
//! | Step | Hook type | Effect |
//! |------|-----------|--------|
//! | `export($hook)` | [`crate::export::ExportRegistrar`] | Prepend export catalog hook |
//! | `cap_attach($Tag, $Cap, $expr)` | — | Prepend `$expr` onto the capability HList ([`CapTagAbsent`](crate::traits::add::CapTagAbsent)) |
//! | `cap_hook($Tag, $Cap, $hook)` | [`CapHookExt`](crate::capability::CapHookExt) registrar | Prepend `$hook` on an existing `$Cap` capability |
//! | `apps($hook)` | [`crate::apps::AppsRegistrar`] | Register dashboard app tile |
//! | `rune_env($hook)` | [`crate::rune_env::RuneEnvRegistrar`] | Register Rune sandbox env |
//! | `migrations($hook)` | [`crate::migration::MigrationRegistrar`] | Queue SeaORM migrations |
//! | `templates($hook)` | [`crate::template::TemplateRegistrar`] | Register minijinja/maud pages |
//! | `templates($path::Hook, $Idx…)` | same | Generic hook with frunk index params |
//! | `slots($hook)` | [`crate::components::SlotRegistrar`] | Register shell slots (topbar, head, …) |
//! | `config($Tag, $Ty)` | — | Append `[plugins.*]` config section with defaults |
//! | `http($hook)` | [`crate::http::RouteRegistrar`] | Queue deferred route mount hook |
//! | `state($hook)` | [`crate::hooks::AttachState`] | Attach plugin state at mount (DB/config deps) |
//! | `seeds($hook)` | [`crate::hooks::RunSeed`] | Run async seed after mount |
//! | `serve_startup($hook)` | [`crate::hooks::RunServeStartup`] | Run hook when `serve` starts (not migrate/seed) |
//! | `commands($hook)` | [`crate::command::CommandRegistrar`] | Register CLI subcommands |
//!
//! `$hook` is a zero-sized type implementing the registrar trait for that capability
//! (typically `Hook` in the plugin's `apps`, `routes`, `templates`, … module).
//!
//! ### Custom capabilities
//!
//! `$Cap` is a type constructor for a local builder struct (e.g. `SidebarCap` in
//! `SidebarCap<Hooks>`). It must implement [`HasCapTag`](crate::capability::HasCapTag),
//! [`Capability`](crate::capability::Capability), and — for `cap_hook` —
//! [`CapHookExt`](crate::capability::CapHookExt). `$expr` in `cap_attach` is usually
//! `MyCap::<frunk::HNil>::new()`.
//!
//! ```ignore
//! // First plugin: attach + base hook
//! cap_attach(SidebarTag, SidebarCap, SidebarCap::<frunk::HNil>::new()),
//! cap_hook(SidebarTag, SidebarCap, sidebar::BaseHook),
//!
//! // Addon plugin: hook only (cap already on the stack)
//! cap_hook(SidebarTag, SidebarCap, accounting_sidebar::Hook),
//! ```
//!
//! ## Generated items
//!
//! - `InstallOutput<L, ...>` — type alias for the post-install capability HList
//! - `install(app) -> InstallOutput<…>` — chains capability mutations; doc comes from the
//!   `///` comment above `steps:`

/// Generate the capability-stack `install` function and intermediate type aliases.
///
/// # Examples
///
/// Full plugin (users):
///
/// ```ignore
/// define_plugin_install! {
///     plugin: UsersTag;
///     /// Register users deferred hooks and config section.
///     steps: [
///         apps(apps::Hook),
///         migrations(migrations::Hook),
///         templates(templates::Hook),
///         slots(templates::SlotsHook),
///         config(UsersConfigTag, UsersConfig),
///         http(routes::Hook),
///         state(StateHook),
///         seeds(SeedsHook),
///         commands(cli::Hook),
///     ]
/// }
/// ```
///
/// Minimal addon (no DB state hook):
///
/// ```ignore
/// define_plugin_install! {
///     plugin: SignupTag;
///     steps: [templates(templates::Hook, LoginIdx, UnauthIdx), http(routes::Hook)]
/// }
/// ```
///
/// Eager passthrough state (dashboard):
///
/// ```ignore
/// define_plugin_install! {
///     plugin: DashboardTag;
///     steps: [templates(templates::Hook), slots(templates::SlotsHook), http(routes::Hook)];
///     finish: add_capability(DashboardStateCap, CapStore::with_items(DashboardState));
/// }
/// ```
///
/// Deployment-local capability (hub attaches, addons hook):
///
/// ```ignore
/// define_plugin_install! {
///     plugin: AccountsTag;
///     steps: [
///         cap_attach(SidebarTag, SidebarCap, SidebarCap::<frunk::HNil>::new()),
///         cap_hook(SidebarTag, SidebarCap, sidebar::BaseHook),
///         apps(apps::Hook),
///         http(routes::Hook),
///     ]
/// }
/// ```
pub use lariv_rs_macros::define_plugin_install;
