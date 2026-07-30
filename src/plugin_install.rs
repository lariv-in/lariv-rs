//! Declarative plugin `install` helper — expands `After*` / `InstallOutput` / `install`.

/// Generate the capability-stack `install` function and intermediate type aliases.
///
/// ```ignore
/// define_plugin_install! {
///     plugin: UsersTag;
///     /// Register users deferred hooks and config section.
///     steps: [
///         apps,
///         migrations,
///         templates,
///         slots,
///         config(UsersConfigTag, UsersConfig),
///         http,
///         state,
///         seeds,
///         commands,
///     ]
/// }
/// ```
///
/// Optional eager state (dashboard):
///
/// ```ignore
/// define_plugin_install! {
///     plugin: DashboardTag;
///     /// docs…
///     steps: [templates, slots, http];
///     finish: add_capability(DashboardStateCap, CapStore::with_items(DashboardState));
/// }
/// ```
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
            steps = [$($out)*]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [apps $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* apps];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [migrations $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* migrations];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [templates $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* templates];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [slots $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* slots];
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
        input = [http $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* http];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [state $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* state];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [seeds $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* seeds];
            input = [$($($rest)*)?]
        }
    };
    (
        @parse_steps
        plugin = $plugin:ty;
        meta = { $($meta:tt)* };
        finish = { $($finish:tt)* };
        out = [$($out:tt)*];
        input = [commands $(, $($rest:tt)*)?]
    ) => {
        $crate::plugin_install::define_plugin_install! {
            @parse_steps
            plugin = $plugin;
            meta = { $($meta)* };
            finish = { $($finish)* };
            out = [$($out)* commands];
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
        steps = []
    ) => {
        type InstallOutput<$($param),*> = $crate::app::App<$prev>;

        $($meta)*
        #[allow(
            clippy::type_complexity,
            reason = "install return type is an HList capability stack; InstallOutput already aliases the shape"
        )]
        pub fn install<$($param),*>(
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
        steps = []
    ) => {
        type InstallOutput<$($param),*> =
            $crate::app::App<::frunk::HCons<$finish_cap, $prev>>;

        $($meta)*
        #[allow(
            clippy::type_complexity,
            reason = "install return type is an HList capability stack; InstallOutput already aliases the shape"
        )]
        pub fn install<$($param),*, Proof>(
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
        steps = [apps $($rest:tt)*]
    ) => {
        type AfterApps<$($param),*, AppsIdx, AppsHooks> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::apps::AppsTag,
            $crate::apps::AppsCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::apps::RegisterAppsHook<$plugin>>, AppsHooks>>,
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
                    $crate::apps::AppsCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::apps::RegisterAppsHook<$plugin>>, AppsHooks>>,
                    AppsIdx,
                    OldValue = $crate::apps::AppsCap<AppsHooks>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::apps::AppsTag, AppsIdx, _>(
                    |cap: $crate::apps::AppsCap<AppsHooks>| {
                        cap.add_hook($crate::apps::RegisterAppsHook::<$plugin>::new())
                    },
                )
            };
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
        steps = [migrations $($rest:tt)*]
    ) => {
        type AfterMigrations<$($param),*, MigIdx, MigHooks, MigItems> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::migration::MigrationTag,
            $crate::migration::MigrationCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::migration::RegisterMigrationsHook<$plugin>>, MigHooks>, MigItems>,
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
                    $crate::migration::MigrationCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::migration::RegisterMigrationsHook<$plugin>>, MigHooks>, MigItems>,
                    MigIdx,
                    OldValue = $crate::migration::MigrationCap<MigHooks, MigItems>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::migration::MigrationTag, MigIdx, _>(
                    |cap: $crate::migration::MigrationCap<MigHooks, MigItems>| {
                        cap.add_hook($crate::migration::RegisterMigrationsHook::<$plugin>::new())
                    },
                )
            };
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
        steps = [templates $($rest:tt)*]
    ) => {
        type AfterTemplates<$($param),*, TplIdx, TplHooks, TplItems> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::template::TemplateTag,
            $crate::template::TemplateCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::template::RegisterTemplatesHook<$plugin>>, TplHooks>, TplItems>,
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
                    $crate::template::TemplateCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::template::RegisterTemplatesHook<$plugin>>, TplHooks>, TplItems>,
                    TplIdx,
                    OldValue = $crate::template::TemplateCap<TplHooks, TplItems>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::template::TemplateTag, TplIdx, _>(
                    |cap: $crate::template::TemplateCap<TplHooks, TplItems>| {
                        cap.add_hook($crate::template::RegisterTemplatesHook::<$plugin>::new())
                    },
                )
            };
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
        steps = [slots $($rest:tt)*]
    ) => {
        type AfterSlots<$($param),*, SlotIdx, SlotHooks, SlotItems> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::components::SlotTag,
            $crate::components::SlotCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::components::RegisterSlotsHook<$plugin>>, SlotHooks>, SlotItems>,
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
                    $crate::components::SlotCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::components::RegisterSlotsHook<$plugin>>, SlotHooks>, SlotItems>,
                    SlotIdx,
                    OldValue = $crate::components::SlotCap<SlotHooks, SlotItems>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::components::SlotTag, SlotIdx, _>(
                    |cap: $crate::components::SlotCap<SlotHooks, SlotItems>| {
                        cap.add_hook($crate::components::RegisterSlotsHook::<$plugin>::new())
                    },
                )
            };
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
        steps = [http $($rest:tt)*]
    ) => {
        type AfterHttp<$($param),*, HttpIdx, HttpHooks, HttpRoutes> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::http::HttpTag,
            $crate::http::HttpCap<
                ::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::hooks::MountRoutesHook<$plugin>>, HttpHooks>,
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
                        ::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::hooks::MountRoutesHook<$plugin>>, HttpHooks>,
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
                        cap.add_hook($crate::hooks::MountRoutesHook::<$plugin>::new())
                    },
                )
            };
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
        steps = [state $($rest:tt)*]
    ) => {
        type AfterStateHooks<$($param),*, StateIdx, StateHooks> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::hooks::StateHooksTag,
            $crate::hooks::StateHooksCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::hooks::WithStateHook<$plugin>>, StateHooks>>,
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
                    $crate::hooks::StateHooksCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::hooks::WithStateHook<$plugin>>, StateHooks>>,
                    StateIdx,
                    OldValue = $crate::hooks::StateHooksCap<StateHooks>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::hooks::StateHooksTag, StateIdx, _>(
                    |cap: $crate::hooks::StateHooksCap<StateHooks>| {
                        cap.add_with_state::<$plugin>()
                    },
                )
            };
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
        steps = [seeds $($rest:tt)*]
    ) => {
        type AfterSeeds<$($param),*, SeedsIdx, SeedHooks> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::hooks::SeedsTag,
            $crate::hooks::SeedsCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::hooks::SeedHook<$plugin>>, SeedHooks>>,
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
                    $crate::hooks::SeedsCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::hooks::SeedHook<$plugin>>, SeedHooks>>,
                    SeedsIdx,
                    OldValue = $crate::hooks::SeedsCap<SeedHooks>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::hooks::SeedsTag, SeedsIdx, _>(
                    |cap: $crate::hooks::SeedsCap<SeedHooks>| {
                        cap.add_seed::<$plugin>()
                    },
                )
            };
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
        steps = [commands $($rest:tt)*]
    ) => {
        type AfterCommands<$($param),*, CmdIdx, CmdHooks, CmdItems> = <$prev as $crate::traits::replace::MapByCapTag<
            $crate::command::CommandTag,
            $crate::command::CommandCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::command::RegisterCommandsHook<$plugin>>, CmdHooks>, CmdItems>,
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
                    $crate::command::CommandCap<::frunk::HCons<$crate::tag::Tagged<$plugin, $crate::command::RegisterCommandsHook<$plugin>>, CmdHooks>, CmdItems>,
                    CmdIdx,
                    OldValue = $crate::command::CommandCap<CmdHooks, CmdItems>,
                >,
            };
            calls = {
                $($calls)*
                .replace_capability::<$crate::command::CommandTag, CmdIdx, _>(
                    |cap: $crate::command::CommandCap<CmdHooks, CmdItems>| {
                        cap.add_hook($crate::command::RegisterCommandsHook::<$plugin>::new())
                    },
                )
            };
            steps = [$($rest)*]
        }
    };
}

pub(crate) use define_plugin_install;
