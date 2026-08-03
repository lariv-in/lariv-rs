//! Compile-time view layer stack for request processing and page data assembly.
//!
//! Layers are an HList whose associated [`FoldLayerData::Data`] type is folded from each
//! layer's [`LayerContrib`] (Option C). Layers run tail-first when registered via
//! [`View::layer`] (prepend), matching capability hook install order.
//!
//! Auth/role layers live in the users plugin; CRUD layers are in this module.
//!
//! # Routes
//!
//! Build a stack with [`view`], prepend layers with [`.layer()`](View::layer), run via
//! [`run_layers`], then render with [`render_from_data`].
//!
//! # Use cases
//!
//! - Compose reusable data-loading middleware (path params, detail load, list query).
//! - Type-check page field requirements against contributed Data tags at compile time.
//! - Short-circuit with redirects or early responses before rendering.
//!
//! # Examples
//!
//! ```rust ignore
//! type UserEditView = View<UserEditPage,
//!     HCons<PathLayer,
//!     HCons<DetailLayer<UserLoader, UserTag>,
//!     HCons<UpdateLayer<UserUpdater, UserTag>,
//!     HNil>>>>;
//!
//! let stack = view::<UserEditPage>()
//!     .layer(PathLayer::names(&["id"]))
//!     .layer(DetailLayer::<UserLoader, UserTag>::new())
//!     .layer(UpdateLayer::<UserUpdater, UserTag>::new());
//! ```

mod create;
mod delete;
mod detail;
mod list;
mod method;
mod patch;
mod path;
mod render;
mod update;

pub use create::{
    CreateEntity, CreateLayer, CreatedIdTag, FormErrorsTag, FormValuesTag, HasCreateState,
    HasFormMaps,
};
pub use delete::{DeleteEntity, DeleteLayer, HasDeleteState};
pub use detail::{DetailLayer, HasLoadState, LoadById};
pub use list::{HasListScope, HasListState, ListLayer, ListQuery, LoadList};
pub use method::MethodLayer;
pub use patch::{FoldFormPatchers, FoldQueryPatchers, FormPatcher, QueryPatcher};
pub use path::{PathLayer, PathMap, PathTag};
pub use render::{html_built_page_or_app_layout, html_built_page_with_slots, render_from_data};
pub use update::{HasFormMapsRef, HasUpdateState, UpdateEntity, UpdateLayer};

use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;

use axum::{
    http::{HeaderMap, Method, Uri},
    response::Response,
};
use frunk::{HCons, HNil, hlist::HList};

use crate::tag::Tagged;

/// What one layer prepends onto the stack Data (often [`HNil`] or a single [`Tagged`]).
pub trait LayerContrib {
    type Contrib: HList;
}

/// Type-level prepend of a contrib HList onto an accumulator.
pub trait PrependToType<Acc> {
    type Output;
}

impl<Acc> PrependToType<Acc> for HNil {
    type Output = Acc;
}

impl<H, T, Acc> PrependToType<Acc> for HCons<H, T>
where
    T: PrependToType<Acc>,
{
    type Output = HCons<H, <T as PrependToType<Acc>>::Output>;
}

/// Value-level prepend matching [`PrependToType`].
pub trait PrependTo<Acc>: Sized {
    type Output;
    fn prepend_to(self, acc: Acc) -> Self::Output;
}

impl<Acc> PrependTo<Acc> for HNil {
    type Output = Acc;
    fn prepend_to(self, acc: Acc) -> Self::Output {
        acc
    }
}

impl<H, T, Acc> PrependTo<Acc> for HCons<H, T>
where
    T: PrependTo<Acc>,
{
    type Output = HCons<H, <T as PrependTo<Acc>>::Output>;
    fn prepend_to(self, acc: Acc) -> Self::Output {
        HCons {
            head: self.head,
            tail: self.tail.prepend_to(acc),
        }
    }
}

/// Fold layer HList → associated Data type (Option C).
pub trait FoldLayerData {
    type Data: HList;
}

impl FoldLayerData for HNil {
    type Data = HNil;
}

impl<L, Tail> FoldLayerData for HCons<L, Tail>
where
    L: LayerContrib,
    Tail: FoldLayerData,
    L::Contrib: PrependToType<Tail::Data>,
    <L::Contrib as PrependToType<Tail::Data>>::Output: HList,
{
    type Data = <L::Contrib as PrependToType<Tail::Data>>::Output;
}

/// Page (or any consumer) builds from the folded Data.
pub trait BuildFromData<Data>: Sized + 'static {
    fn build_from_data(data: &Data) -> Self;
}

/// Runtime step: short-circuit with a Response or continue with extended Acc.
pub enum LayerStep<Acc> {
    Continue(Acc),
    Done(Response),
}

/// Per-request inputs shared by all layers in a stack.
///
/// Path params are populated by the HTTP adapter before layers run. Auth layers may set
/// [`auth_present`](Self::auth_present) for downstream role checks.
#[derive(Clone, Debug)]
pub struct LayerRequest {
    pub method: Method,
    pub uri: Uri,
    pub headers: HeaderMap,
    pub path: PathMap,
    pub query: HashMap<String, String>,
    /// Set by auth layers for downstream role checks (also contributed to Data).
    pub auth_present: bool,
}

impl LayerRequest {
    /// Construct from axum method, URI, and headers (path/query filled by adapter).
    pub fn new(method: Method, uri: Uri, headers: HeaderMap) -> Self {
        Self {
            method,
            uri,
            headers,
            path: PathMap::new(),
            query: HashMap::new(),
            auth_present: false,
        }
    }

    /// Parse a path segment as `i64` (returns `None` on missing or invalid).
    pub fn path_i64(&self, name: &str) -> Option<i64> {
        self.path.get(name)?.parse().ok()
    }

    pub fn path_param(&self, name: &str) -> Option<&str> {
        self.path.get(name).map(String::as_str)
    }

    pub fn with_path_id(mut self, id: i64) -> Self {
        self.path.insert("id".into(), id.to_string());
        self
    }

    pub fn with_path(mut self, name: impl Into<String>, value: impl ToString) -> Self {
        self.path.insert(name.into(), value.to_string());
        self
    }

    pub fn with_query_map(mut self, query: HashMap<String, String>) -> Self {
        self.query = query;
        self
    }
}

/// Runtime layer step. `Ctx` carries plugin state (and users state when needed).
pub trait ViewLayer<Ctx, Acc> {
    type AccOut: HList;

    fn run<'a>(
        &'a self,
        ctx: &'a mut Ctx,
        req: &'a mut LayerRequest,
        acc: Acc,
    ) -> impl Future<Output = LayerStep<Self::AccOut>> + Send + 'a
    where
        Acc: Send + 'a;
}

/// Run a layer HList tail-first (registration / install order).
pub trait RunLayers<Ctx, Acc>: FoldLayerData {
    fn run_all<'a>(
        &'a self,
        ctx: &'a mut Ctx,
        req: &'a mut LayerRequest,
        acc: Acc,
    ) -> impl Future<Output = LayerStep<Self::Data>> + Send + 'a
    where
        Acc: Send + 'a,
        Ctx: Send + 'a;
}

impl<Ctx> RunLayers<Ctx, HNil> for HNil {
    fn run_all<'a>(
        &'a self,
        _ctx: &'a mut Ctx,
        _req: &'a mut LayerRequest,
        acc: HNil,
    ) -> impl Future<Output = LayerStep<Self::Data>> + Send + 'a
    where
        HNil: Send + 'a,
        Ctx: Send + 'a,
    {
        async move { LayerStep::Continue(acc) }
    }
}

impl<Ctx, Acc, L, Tail> RunLayers<Ctx, Acc> for HCons<L, Tail>
where
    Acc: HList + Send,
    Tail: RunLayers<Ctx, Acc> + Sync,
    <Tail as FoldLayerData>::Data: Send,
    L: ViewLayer<Ctx, <Tail as FoldLayerData>::Data> + Sync,
    L::AccOut: HList + Send,
    Self: FoldLayerData<Data = L::AccOut>,
    Ctx: Send,
{
    fn run_all<'a>(
        &'a self,
        ctx: &'a mut Ctx,
        req: &'a mut LayerRequest,
        acc: Acc,
    ) -> impl Future<Output = LayerStep<Self::Data>> + Send + 'a
    where
        Acc: Send + 'a,
        Ctx: Send + 'a,
    {
        async move {
            match self.tail.run_all(ctx, req, acc).await {
                LayerStep::Done(r) => LayerStep::Done(r),
                LayerStep::Continue(mid) => self.head.run(ctx, req, mid).await,
            }
        }
    }
}

/// Run `layers` starting from an empty accumulator.
pub async fn run_layers<L, Ctx>(
    layers: &L,
    ctx: &mut Ctx,
    req: &mut LayerRequest,
) -> LayerStep<L::Data>
where
    L: RunLayers<Ctx, HNil>,
    Ctx: Send,
{
    layers.run_all(ctx, req, HNil).await
}

/// Fluent view stack builder; `.layer()` prepends (runtime order = reverse of type list).
pub struct View<PageTag, Layers> {
    pub layers: Layers,
    _page: PhantomData<fn() -> PageTag>,
}

impl<PageTag> View<PageTag, HNil> {
    pub const fn new() -> Self {
        Self {
            layers: HNil,
            _page: PhantomData,
        }
    }
}

impl<PageTag> Default for View<PageTag, HNil> {
    fn default() -> Self {
        Self::new()
    }
}

impl<PageTag, Layers> View<PageTag, Layers> {
    /// Prepend `layer` onto the stack (runs before previously added layers).
    pub fn layer<L>(self, layer: L) -> View<PageTag, HCons<L, Layers>> {
        View {
            layers: HCons {
                head: layer,
                tail: self.layers,
            },
            _page: PhantomData,
        }
    }

    pub fn layers_ref(&self) -> &Layers {
        &self.layers
    }
}

/// Start an empty view stack for page type `PageTag`.
pub fn view<PageTag>() -> View<PageTag, HNil> {
    View::new()
}

/// Helper: prepend a single tagged value onto Acc.
pub fn cons_tagged<Tag, V, Acc>(value: V, acc: Acc) -> HCons<Tagged<Tag, V>, Acc> {
    HCons {
        head: Tagged::new(value),
        tail: acc,
    }
}

#[cfg(test)]
mod tests {
    use frunk::{HCons, HNil};

    use super::*;
    use crate::tag::Tagged;
    use crate::traits::get::GetByTag;

    struct TagA;
    struct TagB;

    struct LayerA;
    struct LayerB;

    impl LayerContrib for LayerA {
        type Contrib = HCons<Tagged<TagA, u8>, HNil>;
    }

    impl LayerContrib for LayerB {
        type Contrib = HCons<Tagged<TagB, bool>, HNil>;
    }

    #[test]
    fn fold_layer_data_prepending_contribs() {
        // view().layer(A).layer(B) => HCons<B, HCons<A, HNil>>
        type Stack = HCons<LayerB, HCons<LayerA, HNil>>;
        // Data = Contrib_B ++ Contrib_A ++ HNil = Tagged<B,bool> :: Tagged<A,u8> :: HNil
        type Data = <Stack as FoldLayerData>::Data;
        fn assert_data(_d: &Data) {}
        let data: Data = HCons {
            head: Tagged::<TagB, _>::new(true),
            tail: HCons {
                head: Tagged::<TagA, _>::new(7u8),
                tail: HNil,
            },
        };
        assert_data(&data);
        assert!(*GetByTag::<TagB, _>::get_by_tag(&data));
        assert_eq!(*GetByTag::<TagA, _>::get_by_tag(&data), 7u8);
    }

    struct Page {
        a: u8,
        b: bool,
    }

    impl BuildFromData<HCons<Tagged<TagB, bool>, HCons<Tagged<TagA, u8>, HNil>>> for Page {
        fn build_from_data(
            data: &HCons<Tagged<TagB, bool>, HCons<Tagged<TagA, u8>, HNil>>,
        ) -> Self {
            Self {
                a: *GetByTag::<TagA, _>::get_by_tag(data),
                b: *GetByTag::<TagB, _>::get_by_tag(data),
            }
        }
    }

    #[test]
    fn build_from_data_projects_tags() {
        let data = HCons {
            head: Tagged::<TagB, _>::new(false),
            tail: HCons {
                head: Tagged::<TagA, _>::new(3u8),
                tail: HNil,
            },
        };
        let page = Page::build_from_data(&data);
        assert_eq!(page.a, 3);
        assert!(!page.b);
    }

    /// Extra field page: does not require matching a default Generic::Repr.
    struct FancyPage {
        a: u8,
        banner: &'static str,
    }

    impl BuildFromData<HCons<Tagged<TagA, u8>, HNil>> for FancyPage {
        fn build_from_data(data: &HCons<Tagged<TagA, u8>, HNil>) -> Self {
            Self {
                a: *GetByTag::<TagA, _>::get_by_tag(data),
                banner: "extra",
            }
        }
    }

    #[test]
    fn build_from_data_allows_extra_page_fields() {
        let data = HCons {
            head: Tagged::<TagA, _>::new(1u8),
            tail: HNil,
        };
        let page = FancyPage::build_from_data(&data);
        assert_eq!(page.a, 1);
        assert_eq!(page.banner, "extra");
    }

    #[test]
    fn role_layer_allow_is_per_stack() {
        use crate::plugins::users::layers::RoleLayer;
        let editors = RoleLayer::allow(&["editor", "admin"]);
        let admins = RoleLayer::allow(&["admin"]);
        assert_eq!(editors.roles, &["editor", "admin"]);
        assert_eq!(admins.roles, &["admin"]);
    }

    /// Pages may require tags that only exist when a contributing layer is present.
    #[test]
    fn missing_data_tag_is_a_compile_time_concern() {
        // FancyPage only needs TagA. A page that required TagB would not implement
        // BuildFromData for TagA-only Data.
        let _ = std::any::type_name::<FancyPage>();
    }
}
