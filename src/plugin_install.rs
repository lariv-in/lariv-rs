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
//! | `grapesjs($hook)` | [`crate::grapesjs::GrapesJsRegistrar`] | Register GrapesJS blocks/components/traits/themes |
//! | `rune_env($hook)` | [`crate::rune_env::RuneEnvRegistrar`] | Register Rune sandbox env |
//! | `tools($hook)` | [`crate::llm_tools::ToolsRegistrar`] | Register LLM tool handlers |
//! | `migrations($hook)` | [`crate::migration::MigrationRegistrar`] | Queue SeaORM migrations |
//! | `templates($hook)` | [`crate::template::TemplateRegistrar`] | Register minijinja/maud pages |
//! | `templates($path::Hook, $Idx…)` | same | Generic hook with frunk index params |
//! | `slots($hook)` | [`crate::components::SlotRegistrar`] | Register shell slots (topbar, head, …) |
//! | `config($Tag, $Ty)` | — | Append `[plugins.*]` config section with defaults |
//! | `http($hook)` | [`crate::http::RouteRegistrar`] | Queue deferred route mount hook |
//! | `state($hook)` | [`crate::hooks::AttachState`] | Attach plugin state at mount (DB/config deps) |
//! | `seeds($hook)` | [`crate::hooks::RunSeed`] | Run async seed after mount |
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
///     plugin: NoSignupTag;
///     steps: [templates(templates::Hook, LoginIdx), http(routes::Hook)]
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
#[macro_export]
macro_rules! define_plugin_install {
    (
        plugin: $plugin:ty;
        $(#[$meta:meta])*
        steps: [$($steps:tt)*]
        $(; finish: add_capability($finish_cap:ty, $finish_expr:expr))?
        $(;)?
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $(#[$meta])* };
            finish = { $($finish_cap, $finish_expr)? };
            out = [];
            input = [$($steps)*]
        }
    };

    // —— normalize step list (strip commas) ——
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = []
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @step
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = L;
            params = (L);
            bounds = {};
            calls = {};
            install_proofs = [];
            steps = [$($out)*]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [export($hook:path) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* export($hook)];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [cap_attach($cap_tag:ty, $($cap:ident)::+, $cap_expr:expr) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* cap_attach($cap_tag, $($cap)::+, $cap_expr)];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [cap_hook($cap_tag:ty, $($cap:ident)::+, $hook:path) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* cap_hook($cap_tag, $($cap)::+, $hook)];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [apps($hook:path) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* apps($hook)];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [grapesjs($hook:path) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* grapesjs($hook)];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [rune_env($hook:path) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* rune_env($hook)];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [tools($hook:path) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* tools($hook)];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [migrations($hook:path) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* migrations($hook)];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [templates($hook:path) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* templates($hook)];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [templates($($hook_path:ident)::+ $(, $idx:ident)*) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* templates($($hook_path)::+ $(, $idx)*)];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [slots($hook:path) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* slots($hook)];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [config($cfg_tag:ty, $cfg_ty:ty) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* config($cfg_tag, $cfg_ty)];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [http($hook:path) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* http($hook)];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [state($hook:path) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* state($hook)];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [seeds($hook:path) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* seeds($hook)];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [commands($hook:path) $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* commands($hook)];
            input = [$($($rest)*)?]
        }
    };

    // —— done: emit InstallOutput + install ——
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = {};
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = [$($attach_proof:ident),*];
        steps = []
    ) => {
        type InstallOutput<$($param),*> = $crate::app::App<$prev>;

        $($meta)*
        #[allow(
            clippy::type_complexity,
            reason = "install return type is an HList capability stack; InstallOutput already aliases the shape"
        )]
        pub fn install<$($param),* $(, $attach_proof)*>(
            app: $crate::app::App<L>,
        ) -> InstallOutput<$($param),*>
        where
            $($bounds)*
        {
            app $($calls)*
        }
    };
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $finish_cap:ty, $finish_expr:expr };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = [$($attach_proof:ident),*];
        steps = []
    ) => {
        type InstallOutput<$($param),*> =
            $crate::app::App<::frunk::HCons<$finish_cap, $prev>>;

        $($meta)*
        #[allow(
            clippy::type_complexity,
            reason = "install return type is an HList capability stack; InstallOutput already aliases the shape"
        )]
        pub fn install<$($param),* $(, $attach_proof)*, Proof>(
            app: $crate::app::App<L>,
        ) -> InstallOutput<$($param),*>
        where
            $($bounds)*
            $prev: ::frunk::hlist::HList
                + $crate::traits::add::CapTagAbsent<$plugin, Proof>,
        {
            app $($calls)*.add_capability($finish_expr)
        }
    };

    // —— export ——
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = $install_proofs:tt;
        steps = [export($hook:path) $($rest:tt)*]
    ) => {
        type AfterExport<$($param),*, ExportIdx, ExportHooks> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::export::ExportTag,
            $crate::export::ExportCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, ExportHooks>>,
            ExportIdx,
        >>::Output;

        $crate::plugin_install::define_plugin_install! {
            @step
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = AfterExport<$($param),*, ExportIdx, ExportHooks>;
            params = ($($param),*, ExportIdx, ExportHooks);
            bounds = {
                $($bounds)*
                $prev: $crate::traits::get::GetByCapTag<
                    $crate::export::ExportTag,
                    ExportIdx,
                    Value = $crate::export::ExportCap<ExportHooks>,
                >,
                $prev: $crate::traits::replace::MapByCapTag<
                    $crate::export::ExportTag,
                    $crate::export::ExportCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, ExportHooks>>,
                    ExportIdx,
                    OldValue = $crate::export::ExportCap<ExportHooks>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::export::ExportTag, ExportIdx, _>(
                    |cap: $crate::export::ExportCap<ExportHooks>| {
                        cap.add_hook(<$hook>::default())
                    },
                )
            };
            install_proofs = $install_proofs;
            steps = [$($rest)*]
        }
    };

    // —— cap_attach ——
    // Unique proof type-params are derived from the Cap path's last segment so a
    // plugin may attach multiple custom capabilities in one install chain.
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = [$($install_proofs:ident),*];
        steps = [cap_attach($cap_tag:ty, $($cap:ident)::+, $cap_expr:expr) $($rest:tt)*]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @cap_last
            kind = attach;
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = $prev;
            params = ($($param),*);
            bounds = { $($bounds)* };
            calls = { $($calls)* };
            install_proofs = [$($install_proofs),*];
            cap_tag = $cap_tag;
            cap_path = $($cap)::+;
            cap_segs = [$($cap),+];
            cap_expr = $cap_expr;
            hook = _;
            rest = [$($rest)*]
        }
    };

    // —— cap_hook ——
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = [$($install_proofs:ident),*];
        steps = [cap_hook($cap_tag:ty, $($cap:ident)::+, $hook:path) $($rest:tt)*]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @cap_last
            kind = hook;
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = $prev;
            params = ($($param),*);
            bounds = { $($bounds)* };
            calls = { $($calls)* };
            install_proofs = [$($install_proofs),*];
            cap_tag = $cap_tag;
            cap_path = $($cap)::+;
            cap_segs = [$($cap),+];
            cap_expr = _;
            hook = $hook;
            rest = [$($rest)*]
        }
    };

    // Peel Cap path segments until one remains — used as a unique paste suffix.
    (
        @cap_last
        kind = $kind:ident;
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = [$($install_proofs:ident),*];
        cap_tag = $cap_tag:ty;
        cap_path = $($cap:ident)::+;
        cap_segs = [$head:ident, $($tail:ident),+];
        cap_expr = $cap_expr:tt;
        hook = $hook:tt;
        rest = [$($rest:tt)*]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @cap_last
            kind = $kind;
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = $prev;
            params = ($($param),*);
            bounds = { $($bounds)* };
            calls = { $($calls)* };
            install_proofs = [$($install_proofs),*];
            cap_tag = $cap_tag;
            cap_path = $($cap)::+;
            cap_segs = [$($tail),+];
            cap_expr = $cap_expr;
            hook = $hook;
            rest = [$($rest)*]
        }
    };

    (
        @cap_last
        kind = attach;
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = [$($attach_proof:ident),*];
        cap_tag = $cap_tag:ty;
        cap_path = $($cap:ident)::+;
        cap_segs = [$cap_name:ident];
        cap_expr = $cap_expr:expr;
        hook = $hook:tt;
        rest = [$($rest:tt)*]
    ) => {
        $crate::paste::paste! {
            type [<AfterCapAttach_ $cap_name>]<$($param),*> =
                ::frunk::HCons<$($cap)::+<::frunk::HNil>, $prev>;

            $crate::plugin_install::define_plugin_install! {
                @step
                plugin = $plugin;
                meta = { $($meta)* };
                finish = { $($finish)* };
                prev = [<AfterCapAttach_ $cap_name>]<$($param),*>;
                params = ($($param),*);
                bounds = {
                    $($bounds)*
                    $prev: ::frunk::hlist::HList
                        + $crate::traits::add::CapTagAbsent<
                            $cap_tag,
                            [<CapAttachProof_ $cap_name>],
                        >,
                    $($cap)::+<::frunk::HNil>: $crate::capability::HasCapTag<Tag = $cap_tag>,
                };
                calls = {
                    $($calls)*
                    .attach_cap($cap_expr)
                };
                install_proofs = [$($attach_proof,)* [<CapAttachProof_ $cap_name>]];
                steps = [$($rest)*]
            }
        }
    };

    (
        @cap_last
        kind = hook;
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = [$($attach_proof:ident),*];
        cap_tag = $cap_tag:ty;
        cap_path = $($cap:ident)::+;
        cap_segs = [$cap_name:ident];
        cap_expr = $cap_expr:tt;
        hook = $hook:path;
        rest = [$($rest:tt)*]
    ) => {
        $crate::paste::paste! {
            type [<CapTy_ $cap_name>]<[<Hooks_ $cap_name>]> = $($cap)::+<[<Hooks_ $cap_name>]>;

            type [<AfterCapHook_ $cap_name>]<
                $($param),*,
                [<CapIdx_ $cap_name>],
                [<Hooks_ $cap_name>],
            > = <$prev as $crate::traits::replace::MapByCapTag<
                $cap_tag,
                [<CapTy_ $cap_name>]<
                    ::frunk::HCons<
                        $crate::tag::Tagged<$plugin, $hook>,
                        [<Hooks_ $cap_name>],
                    >
                >,
                [<CapIdx_ $cap_name>],
            >>::Output;

            $crate::plugin_install::define_plugin_install! {
                @step
                plugin = $plugin;
                meta = { $($meta)* };
                finish = { $($finish)* };
                prev = [<AfterCapHook_ $cap_name>]<
                    $($param),*,
                    [<CapIdx_ $cap_name>],
                    [<Hooks_ $cap_name>],
                >;
                params = ($($param),*, [<CapIdx_ $cap_name>], [<Hooks_ $cap_name>]);
                bounds = {
                    $($bounds)*
                    $prev: $crate::traits::get::GetByCapTag<
                        $cap_tag,
                        [<CapIdx_ $cap_name>],
                        Value = [<CapTy_ $cap_name>]<[<Hooks_ $cap_name>]>,
                    >,
                    $prev: $crate::traits::replace::MapByCapTag<
                        $cap_tag,
                        [<CapTy_ $cap_name>]<
                            ::frunk::HCons<
                                $crate::tag::Tagged<$plugin, $hook>,
                                [<Hooks_ $cap_name>],
                            >
                        >,
                        [<CapIdx_ $cap_name>],
                        OldValue = [<CapTy_ $cap_name>]<[<Hooks_ $cap_name>]>,
                    >,
                };
                calls = {
                    $($calls)*
                    .replace_capability::<$cap_tag, [<CapIdx_ $cap_name>], _>(
                        |cap: [<CapTy_ $cap_name>]<[<Hooks_ $cap_name>]>| {
                            <[<CapTy_ $cap_name>]<[<Hooks_ $cap_name>]> as $crate::capability::CapHookExt<
                                $plugin,
                                $hook,
                            >>::prepend_cap_hook(
                                cap,
                                <$hook as ::core::default::Default>::default(),
                            )
                        },
                    )
                };
                install_proofs = [$($attach_proof),*];
                steps = [$($rest)*]
            }
        }
    };

    // —— apps ——
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = $install_proofs:tt;
        steps = [apps($hook:path) $($rest:tt)*]
    ) => {
        type AfterApps<$($param),*, AppsIdx, AppsHooks> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::apps::AppsTag,
            $crate::apps::AppsCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, AppsHooks>>,
            AppsIdx,
        >>::Output;

        $crate::plugin_install::define_plugin_install! {
            @step
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = AfterApps<$($param),*, AppsIdx, AppsHooks>;
            params = ($($param),*, AppsIdx, AppsHooks);
            bounds = {
                $($bounds)*
                $prev: $crate::traits::get::GetByCapTag<
                    $crate::apps::AppsTag,
                    AppsIdx,
                    Value = $crate::apps::AppsCap<AppsHooks>,
                >,
                $prev: $crate::traits::replace::MapByCapTag<
                    $crate::apps::AppsTag,
                    $crate::apps::AppsCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, AppsHooks>>,
                    AppsIdx,
                    OldValue = $crate::apps::AppsCap<AppsHooks>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::apps::AppsTag, AppsIdx, _>(
                    |cap: $crate::apps::AppsCap<AppsHooks>| {
                        cap.add_hook(<$hook>::default())
                    },
                )
            };
            install_proofs = $install_proofs;
            steps = [$($rest)*]
        }
    };

    // —— grapesjs ——
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = $install_proofs:tt;
        steps = [grapesjs($hook:path) $($rest:tt)*]
    ) => {
        type AfterGrapesJs<$($param),*, GjsIdx, GjsHooks> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::grapesjs::GrapesJsTag,
            $crate::grapesjs::GrapesJsCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, GjsHooks>>,
            GjsIdx,
        >>::Output;

        $crate::plugin_install::define_plugin_install! {
            @step
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = AfterGrapesJs<$($param),*, GjsIdx, GjsHooks>;
            params = ($($param),*, GjsIdx, GjsHooks);
            bounds = {
                $($bounds)*
                $prev: $crate::traits::get::GetByCapTag<
                    $crate::grapesjs::GrapesJsTag,
                    GjsIdx,
                    Value = $crate::grapesjs::GrapesJsCap<GjsHooks>,
                >,
                $prev: $crate::traits::replace::MapByCapTag<
                    $crate::grapesjs::GrapesJsTag,
                    $crate::grapesjs::GrapesJsCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, GjsHooks>>,
                    GjsIdx,
                    OldValue = $crate::grapesjs::GrapesJsCap<GjsHooks>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::grapesjs::GrapesJsTag, GjsIdx, _>(
                    |cap: $crate::grapesjs::GrapesJsCap<GjsHooks>| {
                        cap.add_hook(<$hook>::default())
                    },
                )
            };
            install_proofs = $install_proofs;
            steps = [$($rest)*]
        }
    };

    // —— rune_env ——
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = $install_proofs:tt;
        steps = [rune_env($hook:path) $($rest:tt)*]
    ) => {
        type AfterRuneEnv<$($param),*, RuneEnvIdx, RuneEnvHooks> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::rune_env::RuneEnvTag,
            $crate::rune_env::RuneEnvCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, RuneEnvHooks>>,
            RuneEnvIdx,
        >>::Output;

        $crate::plugin_install::define_plugin_install! {
            @step
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = AfterRuneEnv<$($param),*, RuneEnvIdx, RuneEnvHooks>;
            params = ($($param),*, RuneEnvIdx, RuneEnvHooks);
            bounds = {
                $($bounds)*
                $prev: $crate::traits::get::GetByCapTag<
                    $crate::rune_env::RuneEnvTag,
                    RuneEnvIdx,
                    Value = $crate::rune_env::RuneEnvCap<RuneEnvHooks>,
                >,
                $prev: $crate::traits::replace::MapByCapTag<
                    $crate::rune_env::RuneEnvTag,
                    $crate::rune_env::RuneEnvCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, RuneEnvHooks>>,
                    RuneEnvIdx,
                    OldValue = $crate::rune_env::RuneEnvCap<RuneEnvHooks>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::rune_env::RuneEnvTag, RuneEnvIdx, _>(
                    |cap: $crate::rune_env::RuneEnvCap<RuneEnvHooks>| {
                        cap.add_hook(<$hook>::default())
                    },
                )
            };
            install_proofs = $install_proofs;
            steps = [$($rest)*]
        }
    };

    // —— tools ——
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = $install_proofs:tt;
        steps = [tools($hook:path) $($rest:tt)*]
    ) => {
        type AfterTools<$($param),*, ToolsIdx, ToolsHooks> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::llm_tools::LlmToolsTag,
            $crate::llm_tools::LlmToolsCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, ToolsHooks>>,
            ToolsIdx,
        >>::Output;

        $crate::plugin_install::define_plugin_install! {
            @step
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = AfterTools<$($param),*, ToolsIdx, ToolsHooks>;
            params = ($($param),*, ToolsIdx, ToolsHooks);
            bounds = {
                $($bounds)*
                $prev: $crate::traits::get::GetByCapTag<
                    $crate::llm_tools::LlmToolsTag,
                    ToolsIdx,
                    Value = $crate::llm_tools::LlmToolsCap<ToolsHooks>,
                >,
                $prev: $crate::traits::replace::MapByCapTag<
                    $crate::llm_tools::LlmToolsTag,
                    $crate::llm_tools::LlmToolsCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, ToolsHooks>>,
                    ToolsIdx,
                    OldValue = $crate::llm_tools::LlmToolsCap<ToolsHooks>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::llm_tools::LlmToolsTag, ToolsIdx, _>(
                    |cap: $crate::llm_tools::LlmToolsCap<ToolsHooks>| {
                        cap.add_hook(<$hook>::default())
                    },
                )
            };
            install_proofs = $install_proofs;
            steps = [$($rest)*]
        }
    };

    // —— migrations ——
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = $install_proofs:tt;
        steps = [migrations($hook:path) $($rest:tt)*]
    ) => {
        type AfterMigrations<$($param),*, MigIdx, MigHooks, MigItems> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::migration::MigrationTag,
            $crate::migration::MigrationCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, MigHooks>, MigItems>,
            MigIdx,
        >>::Output;

        $crate::plugin_install::define_plugin_install! {
            @step
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = AfterMigrations<$($param),*, MigIdx, MigHooks, MigItems>;
            params = ($($param),*, MigIdx, MigHooks, MigItems);
            bounds = {
                $($bounds)*
                $prev: $crate::traits::get::GetByCapTag<
                    $crate::migration::MigrationTag,
                    MigIdx,
                    Value = $crate::migration::MigrationCap<MigHooks, MigItems>,
                >,
                $prev: $crate::traits::replace::MapByCapTag<
                    $crate::migration::MigrationTag,
                    $crate::migration::MigrationCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, MigHooks>, MigItems>,
                    MigIdx,
                    OldValue = $crate::migration::MigrationCap<MigHooks, MigItems>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::migration::MigrationTag, MigIdx, _>(
                    |cap: $crate::migration::MigrationCap<MigHooks, MigItems>| {
                        cap.add_hook(<$hook>::default())
                    },
                )
            };
            install_proofs = $install_proofs;
            steps = [$($rest)*]
        }
    };

    // —— templates ——
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = $install_proofs:tt;
        steps = [templates($hook:path) $($rest:tt)*]
    ) => {
        type AfterTemplates<$($param),*, TplIdx, TplHooks, TplItems> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::template::TemplateTag,
            $crate::template::TemplateCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, TplHooks>, TplItems>,
            TplIdx,
        >>::Output;

        $crate::plugin_install::define_plugin_install! {
            @step
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = AfterTemplates<$($param),*, TplIdx, TplHooks, TplItems>;
            params = ($($param),*, TplIdx, TplHooks, TplItems);
            bounds = {
                $($bounds)*
                $prev: $crate::traits::get::GetByCapTag<
                    $crate::template::TemplateTag,
                    TplIdx,
                    Value = $crate::template::TemplateCap<TplHooks, TplItems>,
                >,
                $prev: $crate::traits::replace::MapByCapTag<
                    $crate::template::TemplateTag,
                    $crate::template::TemplateCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, TplHooks>, TplItems>,
                    TplIdx,
                    OldValue = $crate::template::TemplateCap<TplHooks, TplItems>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::template::TemplateTag, TplIdx, _>(
                    |cap: $crate::template::TemplateCap<TplHooks, TplItems>| {
                        cap.add_hook(<$hook>::default())
                    },
                )
            };
            install_proofs = $install_proofs;
            steps = [$($rest)*]
        }
    };
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = $install_proofs:tt;
        steps = [templates($($hook_path:ident)::+ $(, $idx:ident)*) $($rest:tt)*]
    ) => {
        type TemplateHook<$($idx),*> = $($hook_path)::+<$($idx),*>;

        type AfterTemplates<$($param),*, TplIdx, TplHooks, TplItems $(, $idx)*> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::template::TemplateTag,
            $crate::template::TemplateCap<
                ::frunk::HCons<$crate::tag::Tagged<$plugin, TemplateHook<$($idx),*>>, TplHooks>,
                TplItems,
            >,
            TplIdx,
        >>::Output;

        $crate::plugin_install::define_plugin_install! {
            @step
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = AfterTemplates<$($param),*, TplIdx, TplHooks, TplItems $(, $idx)*>;
            params = ($($param),*, TplIdx, TplHooks, TplItems $(, $idx)*);
            bounds = {
                $($bounds)*
                $prev: $crate::traits::get::GetByCapTag<
                    $crate::template::TemplateTag,
                    TplIdx,
                    Value = $crate::template::TemplateCap<TplHooks, TplItems>,
                >,
                $prev: $crate::traits::replace::MapByCapTag<
                    $crate::template::TemplateTag,
                    $crate::template::TemplateCap<
                        ::frunk::HCons<$crate::tag::Tagged<$plugin, TemplateHook<$($idx),*>>, TplHooks>,
                        TplItems,
                    >,
                    TplIdx,
                    OldValue = $crate::template::TemplateCap<TplHooks, TplItems>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::template::TemplateTag, TplIdx, _>(
                    |cap: $crate::template::TemplateCap<TplHooks, TplItems>| {
                        cap.add_hook(<TemplateHook<$($idx),*>>::default())
                    },
                )
            };
            install_proofs = $install_proofs;
            steps = [$($rest)*]
        }
    };

    // —— slots ——
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = $install_proofs:tt;
        steps = [slots($hook:path) $($rest:tt)*]
    ) => {
        type AfterSlots<$($param),*, SlotIdx, SlotHooks, SlotItems> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::components::SlotTag,
            $crate::components::SlotCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, SlotHooks>, SlotItems>,
            SlotIdx,
        >>::Output;

        $crate::plugin_install::define_plugin_install! {
            @step
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = AfterSlots<$($param),*, SlotIdx, SlotHooks, SlotItems>;
            params = ($($param),*, SlotIdx, SlotHooks, SlotItems);
            bounds = {
                $($bounds)*
                $prev: $crate::traits::get::GetByCapTag<
                    $crate::components::SlotTag,
                    SlotIdx,
                    Value = $crate::components::SlotCap<SlotHooks, SlotItems>,
                >,
                $prev: $crate::traits::replace::MapByCapTag<
                    $crate::components::SlotTag,
                    $crate::components::SlotCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, SlotHooks>, SlotItems>,
                    SlotIdx,
                    OldValue = $crate::components::SlotCap<SlotHooks, SlotItems>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::components::SlotTag, SlotIdx, _>(
                    |cap: $crate::components::SlotCap<SlotHooks, SlotItems>| {
                        cap.add_hook(<$hook>::default())
                    },
                )
            };
            install_proofs = $install_proofs;
            steps = [$($rest)*]
        }
    };

    // —— config ——
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = $install_proofs:tt;
        steps = [config($cfg_tag:ty, $cfg_ty:ty) $($rest:tt)*]
    ) => {
        type AfterConfigs<$($param),*, CfgIdx, Configs> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::config::ConfigTag,
            $crate::config::ConfigCap<::frunk::HNil, ::frunk::HCons<$crate::tag::Tagged<$cfg_tag, $cfg_ty>, Configs>>,
            CfgIdx,
        >>::Output;

        $crate::plugin_install::define_plugin_install! {
            @step
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = AfterConfigs<$($param),*, CfgIdx, Configs>;
            params = ($($param),*, CfgIdx, Configs);
            bounds = {
                $($bounds)*
                $prev: $crate::traits::get::GetByCapTag<
                    $crate::config::ConfigTag,
                    CfgIdx,
                    Value = $crate::config::ConfigCap<::frunk::HNil, Configs>,
                >,
                $prev: $crate::traits::replace::MapByCapTag<
                    $crate::config::ConfigTag,
                    $crate::config::ConfigCap<::frunk::HNil, ::frunk::HCons<$crate::tag::Tagged<$cfg_tag, $cfg_ty>, Configs>>,
                    CfgIdx,
                    OldValue = $crate::config::ConfigCap<::frunk::HNil, Configs>,
                >,
                Configs: ::frunk::hlist::HList,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::config::ConfigTag, CfgIdx, _>(
                    |cap: $crate::config::ConfigCap<::frunk::HNil, Configs>| {
                        cap.map_items(|items| ::frunk::HCons {
                            head: $crate::tag::Tagged::new(<$cfg_ty>::default()),
                            tail: items,
                        })
                    },
                )
            };
            install_proofs = $install_proofs;
            steps = [$($rest)*]
        }
    };

    // —— http ——
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = $install_proofs:tt;
        steps = [http($hook:path) $($rest:tt)*]
    ) => {
        type AfterHttp<$($param),*, HttpIdx, HttpHooks, HttpRoutes> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::http::HttpTag,
            $crate::http::HttpCap<
                ::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, HttpHooks>,
                $crate::http::HttpCapability<HttpRoutes>,
            >,
            HttpIdx,
        >>::Output;

        $crate::plugin_install::define_plugin_install! {
            @step
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = AfterHttp<$($param),*, HttpIdx, HttpHooks, HttpRoutes>;
            params = ($($param),*, HttpIdx, HttpHooks, HttpRoutes);
            bounds = {
                $($bounds)*
                $prev: $crate::traits::get::GetByCapTag<
                    $crate::http::HttpTag,
                    HttpIdx,
                    Value = $crate::http::HttpCap<HttpHooks, $crate::http::HttpCapability<HttpRoutes>>,
                >,
                $prev: $crate::traits::replace::MapByCapTag<
                    $crate::http::HttpTag,
                    $crate::http::HttpCap<
                        ::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, HttpHooks>,
                        $crate::http::HttpCapability<HttpRoutes>,
                    >,
                    HttpIdx,
                    OldValue = $crate::http::HttpCap<HttpHooks, $crate::http::HttpCapability<HttpRoutes>>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::http::HttpTag, HttpIdx, _>(
                    |cap: $crate::http::HttpCap<HttpHooks, $crate::http::HttpCapability<HttpRoutes>>| {
                        cap.add_hook(<$hook>::default())
                    },
                )
            };
            install_proofs = $install_proofs;
            steps = [$($rest)*]
        }
    };

    // —— state ——
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = $install_proofs:tt;
        steps = [state($hook:path) $($rest:tt)*]
    ) => {
        type AfterStateHooks<$($param),*, StateIdx, StateHooks> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::hooks::StateHooksTag,
            $crate::hooks::StateHooksCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, StateHooks>>,
            StateIdx,
        >>::Output;

        $crate::plugin_install::define_plugin_install! {
            @step
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = AfterStateHooks<$($param),*, StateIdx, StateHooks>;
            params = ($($param),*, StateIdx, StateHooks);
            bounds = {
                $($bounds)*
                $prev: $crate::traits::get::GetByCapTag<
                    $crate::hooks::StateHooksTag,
                    StateIdx,
                    Value = $crate::hooks::StateHooksCap<StateHooks>,
                >,
                $prev: $crate::traits::replace::MapByCapTag<
                    $crate::hooks::StateHooksTag,
                    $crate::hooks::StateHooksCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, StateHooks>>,
                    StateIdx,
                    OldValue = $crate::hooks::StateHooksCap<StateHooks>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::hooks::StateHooksTag, StateIdx, _>(
                    |cap: $crate::hooks::StateHooksCap<StateHooks>| {
                        cap.add_hook(<$hook>::default())
                    },
                )
            };
            install_proofs = $install_proofs;
            steps = [$($rest)*]
        }
    };

    // —— seeds ——
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = $install_proofs:tt;
        steps = [seeds($hook:path) $($rest:tt)*]
    ) => {
        type AfterSeeds<$($param),*, SeedsIdx, SeedHooks> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::hooks::SeedsTag,
            $crate::hooks::SeedsCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, SeedHooks>>,
            SeedsIdx,
        >>::Output;

        $crate::plugin_install::define_plugin_install! {
            @step
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = AfterSeeds<$($param),*, SeedsIdx, SeedHooks>;
            params = ($($param),*, SeedsIdx, SeedHooks);
            bounds = {
                $($bounds)*
                $prev: $crate::traits::get::GetByCapTag<
                    $crate::hooks::SeedsTag,
                    SeedsIdx,
                    Value = $crate::hooks::SeedsCap<SeedHooks>,
                >,
                $prev: $crate::traits::replace::MapByCapTag<
                    $crate::hooks::SeedsTag,
                    $crate::hooks::SeedsCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, SeedHooks>>,
                    SeedsIdx,
                    OldValue = $crate::hooks::SeedsCap<SeedHooks>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::hooks::SeedsTag, SeedsIdx, _>(
                    |cap: $crate::hooks::SeedsCap<SeedHooks>| {
                        cap.add_hook(<$hook>::default())
                    },
                )
            };
            install_proofs = $install_proofs;
            steps = [$($rest)*]
        }
    };

    // —— commands ——
    (
        @step
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        prev = $prev:ty;
        params = ($($param:ident),*);
        bounds = { $($bounds:tt)* };
        calls = { $($calls:tt)* };
        install_proofs = $install_proofs:tt;
        steps = [commands($hook:path) $($rest:tt)*]
    ) => {
        type AfterCommands<$($param),*, CmdIdx, CmdHooks, CmdItems> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::command::CommandTag,
            $crate::command::CommandCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, CmdHooks>, CmdItems>,
            CmdIdx,
        >>::Output;

        $crate::plugin_install::define_plugin_install! {
            @step
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            prev = AfterCommands<$($param),*, CmdIdx, CmdHooks, CmdItems>;
            params = ($($param),*, CmdIdx, CmdHooks, CmdItems);
            bounds = {
                $($bounds)*
                $prev: $crate::traits::get::GetByCapTag<
                    $crate::command::CommandTag,
                    CmdIdx,
                    Value = $crate::command::CommandCap<CmdHooks, CmdItems>,
                >,
                $prev: $crate::traits::replace::MapByCapTag<
                    $crate::command::CommandTag,
                    $crate::command::CommandCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $hook>, CmdHooks>, CmdItems>,
                    CmdIdx,
                    OldValue = $crate::command::CommandCap<CmdHooks, CmdItems>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::command::CommandTag, CmdIdx, _>(
                    |cap: $crate::command::CommandCap<CmdHooks, CmdItems>| {
                        cap.add_hook(<$hook>::default())
                    },
                )
            };
            install_proofs = $install_proofs;
            steps = [$($rest)*]
        }
    };
}

pub use crate::define_plugin_install;