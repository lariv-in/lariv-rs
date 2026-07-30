//! Strip users signup routes from the HTTP capability (Go `p_no_signup` route patch).

use frunk::hlist::HList;

use crate::{
    http::{HttpCapability, MountRoutes, RegisterRoutes, Route},
    plugins::users::routes::{UsersSignupGetRouteTag, UsersSignupPostRouteTag},
    traits::remove::PluckByTag,
};

use super::NoSignupTag;

impl<R, Templates, Slots, GetIdx, PostIdx>
    RegisterRoutes<NoSignupTag, Templates, Slots, (GetIdx, PostIdx)> for HttpCapability<R>
where
    R: PluckByTag<UsersSignupGetRouteTag, GetIdx, Value = Route>,
    <R as PluckByTag<UsersSignupGetRouteTag, GetIdx>>::Remainder:
        PluckByTag<UsersSignupPostRouteTag, PostIdx, Value = Route>,
    <<R as PluckByTag<UsersSignupGetRouteTag, GetIdx>>::Remainder as PluckByTag<
        UsersSignupPostRouteTag,
        PostIdx,
    >>::Remainder: HList + MountRoutes + Clone,
    Templates: Clone + Send + Sync + 'static,
    Slots: Clone + Send + Sync + 'static,
{
    type Output = HttpCapability<
        <<R as PluckByTag<UsersSignupGetRouteTag, GetIdx>>::Remainder as PluckByTag<
            UsersSignupPostRouteTag,
            PostIdx,
        >>::Remainder,
    >;

    fn register_routes(self) -> Self::Output {
        let (_, after_get) = self.routes.pluck_by_tag();
        let (_, after_post) = after_get.pluck_by_tag();
        HttpCapability {
            routes: after_post,
        }
    }
}
