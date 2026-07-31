//! Declarative plugin HTTP route registration — tags, proof tuple, and [`RouteRegistrar`].

/// Generate route tags, an optional proof type, and a [`RouteRegistrar`] hook impl.
///
/// ```ignore
/// define_plugin_routes! {
///     plugin: FilesystemTag;
///     proof: FilesystemRoutesProof;
///     pages: [
///         pane ListIdx, ListP => VNodeListPageTag, VNodeListPage;
///         page SelectIdx, SelectP => VNodeSelectPageTag, VNodeSelectPage;
///     ];
///     routes: [
///         post VNodeDeletePostRouteTag, "/filesystem/{id}/delete",
///             bare handlers::nodes::delete_post;
///         get VNodeListRouteTag, "/filesystem", handlers::nodes::list;
///     ]
/// }
/// ```
///
/// - `pane` pages require [`RenderAppPane`]; `page` pages are template-only.
/// - Handlers default to `handler::<Templates, Slots, _, _>`; prefix with `bare` for a raw fn.
/// - Omit `pages` / `proof` (or use empty `pages: [];`) for plugins with no template pages.
/// - `slots: clone;` skips [`FoldSlots`] (PWA); default is fold-capable slots.
#[macro_export]
macro_rules! define_plugin_routes {
    (
        plugin: $plugin:ty;
        $(proof: $proof:ident;)?
        $(slots: $slots_mode:ident;)?
        $(pages: [$($pages:tt)*];)?
        routes: [$($routes:tt)*]
        $(;)?
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @parse_pages
            plugin = $plugin;
            proof = { $($proof)? };
            slots = { $($slots_mode)? };
            pages_out = [];
            pages_in = [$($($pages)*)?];
            routes_in = [$($routes)*]
        }
    };

    // —— parse pages ——
    (
        @parse_pages
        plugin = $plugin:ty;
        proof = { $($proof:tt)* };
        slots = { $($slots:tt)* };
        pages_out = [$($out:tt)*];
        pages_in = [];
        routes_in = [$($routes:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @parse_routes
            plugin = $plugin;
            proof = { $($proof)* };
            slots = { $($slots)* };
            pages = [$($out)*];
            routes_out = [];
            routes_in = [$($routes)*]
        }
    };
    (
        @parse_pages
        plugin = $plugin:ty;
        proof = { $($proof:tt)* };
        slots = { $($slots:tt)* };
        pages_out = [$($out:tt)*];
        pages_in = [pane $idx:ident, $p:ident => $page_tag:ty, $page_ty:ty; $($rest:tt)*];
        routes_in = [$($routes:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @parse_pages
            plugin = $plugin;
            proof = { $($proof)* };
            slots = { $($slots)* };
            pages_out = [$($out)* pane($idx, $p, $page_tag, $page_ty)];
            pages_in = [$($rest)*];
            routes_in = [$($routes)*]
        }
    };
    (
        @parse_pages
        plugin = $plugin:ty;
        proof = { $($proof:tt)* };
        slots = { $($slots:tt)* };
        pages_out = [$($out:tt)*];
        pages_in = [page $idx:ident, $p:ident => $page_tag:ty, $page_ty:ty; $($rest:tt)*];
        routes_in = [$($routes:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @parse_pages
            plugin = $plugin;
            proof = { $($proof)* };
            slots = { $($slots)* };
            pages_out = [$($out)* page($idx, $p, $page_tag, $page_ty)];
            pages_in = [$($rest)*];
            routes_in = [$($routes)*]
        }
    };

    // —— parse routes ——
    (
        @parse_routes
        plugin = $plugin:ty;
        proof = { $($proof:tt)* };
        slots = { $($slots:tt)* };
        pages = [$($pages:tt)*];
        routes_out = [$($out:tt)*];
        routes_in = []
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @emit
            plugin = $plugin;
            proof = { $($proof)* };
            slots = { $($slots)* };
            pages = [$($pages)*];
            routes = [$($out)*]
        }
    };
    (
        @parse_routes
        plugin = $plugin:ty;
        proof = { $($proof:tt)* };
        slots = { $($slots:tt)* };
        pages = [$($pages:tt)*];
        routes_out = [$($out:tt)*];
        routes_in = [get $tag:ident, $path:literal, bare $($handler:ident)::+; $($rest:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @parse_routes
            plugin = $plugin;
            proof = { $($proof)* };
            slots = { $($slots)* };
            pages = [$($pages)*];
            routes_out = [$($out)* get_bare($tag, $path, [$($handler)::+])];
            routes_in = [$($rest)*]
        }
    };
    (
        @parse_routes
        plugin = $plugin:ty;
        proof = { $($proof:tt)* };
        slots = { $($slots:tt)* };
        pages = [$($pages:tt)*];
        routes_out = [$($out:tt)*];
        routes_in = [post $tag:ident, $path:literal, bare $($handler:ident)::+; $($rest:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @parse_routes
            plugin = $plugin;
            proof = { $($proof)* };
            slots = { $($slots)* };
            pages = [$($pages)*];
            routes_out = [$($out)* post_bare($tag, $path, [$($handler)::+])];
            routes_in = [$($rest)*]
        }
    };
    (
        @parse_routes
        plugin = $plugin:ty;
        proof = { $($proof:tt)* };
        slots = { $($slots:tt)* };
        pages = [$($pages:tt)*];
        routes_out = [$($out:tt)*];
        routes_in = [get $tag:ident, $path:literal, $($handler:ident)::+; $($rest:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @parse_routes
            plugin = $plugin;
            proof = { $($proof)* };
            slots = { $($slots)* };
            pages = [$($pages)*];
            routes_out = [$($out)* get($tag, $path, [$($handler)::+])];
            routes_in = [$($rest)*]
        }
    };
    (
        @parse_routes
        plugin = $plugin:ty;
        proof = { $($proof:tt)* };
        slots = { $($slots:tt)* };
        pages = [$($pages:tt)*];
        routes_out = [$($out:tt)*];
        routes_in = [post $tag:ident, $path:literal, $($handler:ident)::+; $($rest:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @parse_routes
            plugin = $plugin;
            proof = { $($proof)* };
            slots = { $($slots)* };
            pages = [$($pages)*];
            routes_out = [$($out)* post($tag, $path, [$($handler)::+])];
            routes_in = [$($rest)*]
        }
    };

    // —— emit: no template pages ——
    (
        @emit
        plugin = $plugin:ty;
        proof = { $($proof:tt)* };
        slots = { $($slots:tt)* };
        pages = [];
        routes = [$($routes:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! { @emit_tags $($routes)* }

        $crate::plugin_routes::define_plugin_routes! {
            @emit_impl_no_pages
            plugin = $plugin;
            slots = { $($slots)* };
            routes = [$($routes)*]
        }
    };

    // —— emit: with template pages ——
    (
        @emit
        plugin = $plugin:ty;
        proof = { $proof:ident };
        slots = { $($slots:tt)* };
        pages = [$($pages:tt)+];
        routes = [$($routes:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! { @emit_tags $($routes)* }

        $crate::plugin_routes::define_plugin_routes! {
            @emit_proof
            proof = $proof;
            pages = [$($pages)*]
        }

        $crate::plugin_routes::define_plugin_routes! {
            @collect_bounds
            plugin = $plugin;
            proof = $proof;
            pages_left = [$($pages)*];
            params = {};
            getby = {};
            bounds = {};
            routes = [$($routes)*]
        }
    };

    (
        @collect_bounds
        plugin = $plugin:ty;
        proof = $proof:ident;
        pages_left = [pane ($idx:ident, $p:ident, $page_tag:ty, $page_ty:ty) $($rest:tt)*];
        params = { $($params:tt)* };
        getby = { $($getby:tt)* };
        bounds = { $($bounds:tt)* };
        routes = [$($routes:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @collect_bounds
            plugin = $plugin;
            proof = $proof;
            pages_left = [$($rest)*];
            params = { $($params)* $idx, $p, };
            getby = {
                $($getby)*
                + $crate::traits::get::GetByTag<$page_tag, $idx, Value = $crate::template::TemplateOf<$p>>
            };
            bounds = {
                $($bounds)*
                $idx: 'static,
                $p: ::frunk::Generic<Repr = <$page_ty as ::frunk::Generic>::Repr>
                    + $crate::template::RenderTemplate
                    + $crate::template::RenderAppPane
                    + 'static,
            };
            routes = [$($routes)*]
        }
    };
    (
        @collect_bounds
        plugin = $plugin:ty;
        proof = $proof:ident;
        pages_left = [page ($idx:ident, $p:ident, $page_tag:ty, $page_ty:ty) $($rest:tt)*];
        params = { $($params:tt)* };
        getby = { $($getby:tt)* };
        bounds = { $($bounds:tt)* };
        routes = [$($routes:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @collect_bounds
            plugin = $plugin;
            proof = $proof;
            pages_left = [$($rest)*];
            params = { $($params)* $idx, $p, };
            getby = {
                $($getby)*
                + $crate::traits::get::GetByTag<$page_tag, $idx, Value = $crate::template::TemplateOf<$p>>
            };
            bounds = {
                $($bounds)*
                $idx: 'static,
                $p: ::frunk::Generic<Repr = <$page_ty as ::frunk::Generic>::Repr>
                    + $crate::template::RenderTemplate
                    + 'static,
            };
            routes = [$($routes)*]
        }
    };
    (
        @collect_bounds
        plugin = $plugin:ty;
        proof = $proof:ident;
        pages_left = [];
        params = { $($params:tt)* };
        getby = { $($getby:tt)* };
        bounds = { $($bounds:tt)* };
        routes = [$($routes:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @rev_routes
            plugin = $plugin;
            proof = { $proof<$($params)*> };
            slots = { fold };
            params = { $($params)* };
            getby = { $($getby)* };
            bounds = { $($bounds)* };
            routes = [$($routes)*];
            rev = [];
            fwd = [$($routes)*]
        }
    };

    // —— route tag structs ——
    (@emit_tags) => {};
    (@emit_tags get($tag:ident, $path:literal, [$($handler:tt)*]) $($rest:tt)*) => {
        pub struct $tag;
        $crate::plugin_routes::define_plugin_routes! { @emit_tags $($rest)* }
    };
    (@emit_tags post($tag:ident, $path:literal, [$($handler:tt)*]) $($rest:tt)*) => {
        pub struct $tag;
        $crate::plugin_routes::define_plugin_routes! { @emit_tags $($rest)* }
    };
    (@emit_tags get_bare($tag:ident, $path:literal, [$($handler:tt)*]) $($rest:tt)*) => {
        pub struct $tag;
        $crate::plugin_routes::define_plugin_routes! { @emit_tags $($rest)* }
    };
    (@emit_tags post_bare($tag:ident, $path:literal, [$($handler:tt)*]) $($rest:tt)*) => {
        pub struct $tag;
        $crate::plugin_routes::define_plugin_routes! { @emit_tags $($rest)* }
    };

    // —— proof type alias ——
    (
        @emit_proof
        proof = $proof:ident;
        pages = [$($kind:ident ($idx:ident, $p:ident, $page_tag:ty, $page_ty:ty))*]
    ) => {
        #[allow(
            clippy::type_complexity,
            reason = "one (Idx, P) pair per templated page carried through this plugin's routes"
        )]
        type $proof<$($idx, $p),*> = ($(($idx, $p),)*);
    };

    // —— RegisterRoutes: no pages ——
    (
        @emit_impl_no_pages
        plugin = $plugin:ty;
        slots = { clone };
        routes = [$($routes:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @rev_routes
            plugin = $plugin;
            proof = { () };
            slots = { clone };
            params = {};
            getby = {};
            bounds = {};
            routes = [$($routes)*];
            rev = [];
            fwd = [$($routes)*]
        }
    };
    (
        @emit_impl_no_pages
        plugin = $plugin:ty;
        slots = { $($slots:tt)* };
        routes = [$($routes:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @rev_routes
            plugin = $plugin;
            proof = { () };
            slots = { fold };
            params = {};
            getby = {};
            bounds = {};
            routes = [$($routes)*];
            rev = [];
            fwd = [$($routes)*]
        }
    };

    // Reverse route tags so HList head matches last `prepend` (concrete Output, not `impl Trait`).
    (
        @rev_routes
        plugin = $plugin:ty;
        proof = { $($proof:tt)* };
        slots = { $($slots:tt)* };
        params = { $($params:tt)* };
        getby = { $($getby:tt)* };
        bounds = { $($bounds:tt)* };
        routes = [$($routes:tt)*];
        rev = [$($rev_tag:ident),*];
        fwd = [get($tag:ident, $path:literal, [$($handler:tt)*]) $($rest:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @rev_routes
            plugin = $plugin;
            proof = { $($proof)* };
            slots = { $($slots)* };
            params = { $($params)* };
            getby = { $($getby)* };
            bounds = { $($bounds)* };
            routes = [$($routes)*];
            rev = [$tag $(, $rev_tag)*];
            fwd = [$($rest)*]
        }
    };
    (
        @rev_routes
        plugin = $plugin:ty;
        proof = { $($proof:tt)* };
        slots = { $($slots:tt)* };
        params = { $($params:tt)* };
        getby = { $($getby:tt)* };
        bounds = { $($bounds:tt)* };
        routes = [$($routes:tt)*];
        rev = [$($rev_tag:ident),*];
        fwd = [post($tag:ident, $path:literal, [$($handler:tt)*]) $($rest:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @rev_routes
            plugin = $plugin;
            proof = { $($proof)* };
            slots = { $($slots)* };
            params = { $($params)* };
            getby = { $($getby)* };
            bounds = { $($bounds)* };
            routes = [$($routes)*];
            rev = [$tag $(, $rev_tag)*];
            fwd = [$($rest)*]
        }
    };
    (
        @rev_routes
        plugin = $plugin:ty;
        proof = { $($proof:tt)* };
        slots = { $($slots:tt)* };
        params = { $($params:tt)* };
        getby = { $($getby:tt)* };
        bounds = { $($bounds:tt)* };
        routes = [$($routes:tt)*];
        rev = [$($rev_tag:ident),*];
        fwd = [get_bare($tag:ident, $path:literal, [$($handler:tt)*]) $($rest:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @rev_routes
            plugin = $plugin;
            proof = { $($proof)* };
            slots = { $($slots)* };
            params = { $($params)* };
            getby = { $($getby)* };
            bounds = { $($bounds)* };
            routes = [$($routes)*];
            rev = [$tag $(, $rev_tag)*];
            fwd = [$($rest)*]
        }
    };
    (
        @rev_routes
        plugin = $plugin:ty;
        proof = { $($proof:tt)* };
        slots = { $($slots:tt)* };
        params = { $($params:tt)* };
        getby = { $($getby:tt)* };
        bounds = { $($bounds:tt)* };
        routes = [$($routes:tt)*];
        rev = [$($rev_tag:ident),*];
        fwd = [post_bare($tag:ident, $path:literal, [$($handler:tt)*]) $($rest:tt)*]
    ) => {
        $crate::plugin_routes::define_plugin_routes! {
            @rev_routes
            plugin = $plugin;
            proof = { $($proof)* };
            slots = { $($slots)* };
            params = { $($params)* };
            getby = { $($getby)* };
            bounds = { $($bounds)* };
            routes = [$($routes)*];
            rev = [$tag $(, $rev_tag)*];
            fwd = [$($rest)*]
        }
    };
    (
        @rev_routes
        plugin = $plugin:ty;
        proof = { $($proof:tt)* };
        slots = { clone };
        params = { $($params:tt)* };
        getby = { $($getby:tt)* };
        bounds = { $($bounds:tt)* };
        routes = [$($routes:tt)*];
        rev = [$($rev_tag:ident),+];
        fwd = []
    ) => {
        #[derive(Clone, Copy, Default)]
        pub struct Hook;

        #[allow(
            clippy::type_complexity,
            reason = "HList![…] of this plugin's routes plus prior plugins' R"
        )]
        impl<R, Templates, Slots, $($params)*>
            $crate::http::RouteRegistrar<
                $crate::http::HttpCapability<R>,
                Templates,
                Slots,
                $($proof)*,
            > for Hook
        where
            R: ::frunk::hlist::HList + Clone + $crate::http::MountRoutes,
            Templates: Clone + Send + Sync + 'static $($getby)*,
            $($bounds)*
            Slots: Clone + Send + Sync + 'static,
        {
            type Output = $crate::http::HttpCapability<
                ::frunk::HList![
                    $($crate::tag::Tagged<$rev_tag, $crate::http::Route>,)+
                    ...R
                ],
            >;

            fn register_routes(
                self,
                http: $crate::http::HttpCapability<R>,
            ) -> Self::Output {
                $crate::plugin_routes::define_plugin_routes! { @chain http; $($routes)* }
            }
        }
    };
    (
        @rev_routes
        plugin = $plugin:ty;
        proof = { $($proof:tt)* };
        slots = { fold };
        params = { $($params:tt)* };
        getby = { $($getby:tt)* };
        bounds = { $($bounds:tt)* };
        routes = [$($routes:tt)*];
        rev = [$($rev_tag:ident),+];
        fwd = []
    ) => {
        #[derive(Clone, Copy, Default)]
        pub struct Hook;

        #[allow(
            clippy::type_complexity,
            reason = "HList![…] of this plugin's routes plus prior plugins' R"
        )]
        impl<R, Templates, Slots, $($params)*>
            $crate::http::RouteRegistrar<
                $crate::http::HttpCapability<R>,
                Templates,
                Slots,
                $($proof)*,
            > for Hook
        where
            R: ::frunk::hlist::HList + Clone + $crate::http::MountRoutes,
            Templates: Clone + Send + Sync + 'static $($getby)*,
            $($bounds)*
            Slots: $crate::components::FoldSlots + Clone + Send + Sync + 'static,
        {
            type Output = $crate::http::HttpCapability<
                ::frunk::HList![
                    $($crate::tag::Tagged<$rev_tag, $crate::http::Route>,)+
                    ...R
                ],
            >;

            fn register_routes(
                self,
                http: $crate::http::HttpCapability<R>,
            ) -> Self::Output {
                $crate::plugin_routes::define_plugin_routes! { @chain http; $($routes)* }
            }
        }
    };

    // —— method-chain body ——
    (@chain $acc:expr;) => { $acc };
    (@chain $acc:expr; get($tag:ident, $path:literal, [$($handler:tt)*]) $($rest:tt)*) => {
        $crate::plugin_routes::define_plugin_routes! {
            @chain
            $acc.prepend::<$tag>($crate::http::Route::get(
                $path,
                $($handler)*::<Templates, Slots, _, _>,
            ));
            $($rest)*
        }
    };
    (@chain $acc:expr; post($tag:ident, $path:literal, [$($handler:tt)*]) $($rest:tt)*) => {
        $crate::plugin_routes::define_plugin_routes! {
            @chain
            $acc.prepend::<$tag>($crate::http::Route::post(
                $path,
                $($handler)*::<Templates, Slots, _, _>,
            ));
            $($rest)*
        }
    };
    (@chain $acc:expr; get_bare($tag:ident, $path:literal, [$($handler:tt)*]) $($rest:tt)*) => {
        $crate::plugin_routes::define_plugin_routes! {
            @chain
            $acc.prepend::<$tag>($crate::http::Route::get($path, $($handler)*));
            $($rest)*
        }
    };
    (@chain $acc:expr; post_bare($tag:ident, $path:literal, [$($handler:tt)*]) $($rest:tt)*) => {
        $crate::plugin_routes::define_plugin_routes! {
            @chain
            $acc.prepend::<$tag>($crate::http::Route::post($path, $($handler)*));
            $($rest)*
        }
    };
}

pub use crate::define_plugin_routes;
