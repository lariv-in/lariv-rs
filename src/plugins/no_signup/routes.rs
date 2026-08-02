//! Strip users signup routes from the HTTP capability (Go `p_no_signup` route patch).

use frunk::hlist::HList;

use crate::{
    http::{HttpCapability, MountRoutes, Route, RouteRegistrar},
    plugins::users::routes::{UsersSignupGetRouteTag, UsersSignupPostRouteTag},
    traits::remove::PluckByTag,
};

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl<R, GetIdx, PostIdx> RouteRegistrar<HttpCapability<R>, (GetIdx, PostIdx)> for Hook
where
    R: PluckByTag<UsersSignupGetRouteTag, GetIdx, Value = Route>,
    <R as PluckByTag<UsersSignupGetRouteTag, GetIdx>>::Remainder:
        PluckByTag<UsersSignupPostRouteTag, PostIdx, Value = Route>,
    <<R as PluckByTag<UsersSignupGetRouteTag, GetIdx>>::Remainder as PluckByTag<
        UsersSignupPostRouteTag,
        PostIdx,
    >>::Remainder: HList + MountRoutes + Clone,
{
    type Output = HttpCapability<
        <<R as PluckByTag<UsersSignupGetRouteTag, GetIdx>>::Remainder as PluckByTag<
            UsersSignupPostRouteTag,
            PostIdx,
        >>::Remainder,
    >;

    fn register_routes(self, http: HttpCapability<R>) -> Self::Output {
        let (_, after_get) = http.routes.pluck_by_tag();
        let (_, after_post) = after_get.pluck_by_tag();
        HttpCapability {
            routes: after_post,
        }
    }
}
