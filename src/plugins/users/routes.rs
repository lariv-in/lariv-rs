//! Users HTTP routes — tagged entries on [`HttpCapability`]'s route HList.

use crate::plugin_routes::define_plugin_routes;

use super::{
    UsersTag, handlers,
    templates::{
        ChangePasswordPage, ConfirmDeletePage, LoginPage, RoleCreateModalPage, RoleDetailPage,
        RoleFormPage, RoleListPage, RoleSelectPage, SelfDetailPage, SelfEditPage, SignupPage,
        UnauthenticatedPage, UserDetailPage, UserFormPage, UserListPage, UserSelectPage,
        UsersChangePasswordPageTag, UsersConfirmDeletePageTag, UsersLoginPageTag,
        UsersRoleCreateModalPageTag, UsersRoleDetailPageTag, UsersRoleFormPageTag,
        UsersRoleListPageTag, UsersRoleSelectPageTag, UsersSelfDetailPageTag, UsersSelfEditPageTag,
        UsersSignupPageTag, UsersUnauthenticatedPageTag, UsersUserDetailPageTag,
        UsersUserFormPageTag, UsersUserListPageTag, UsersUserSelectPageTag,
    },
};

define_plugin_routes! {
    plugin: UsersTag;
    proof: UsersRoutesProof;
    pages: [
        pane LoginIdx, LoginP => UsersLoginPageTag, LoginPage;
        pane SignupIdx, SignupP => UsersSignupPageTag, SignupPage;
        pane UnauthIdx, UnauthP => UsersUnauthenticatedPageTag, UnauthenticatedPage;
        pane SelfDetailIdx, SelfDetailP => UsersSelfDetailPageTag, SelfDetailPage;
        pane SelfEditIdx, SelfEditP => UsersSelfEditPageTag, SelfEditPage;
        pane ChangePasswordIdx, ChangePasswordP => UsersChangePasswordPageTag, ChangePasswordPage;
        pane UserListIdx, UserListP => UsersUserListPageTag, UserListPage;
        pane UserFormIdx, UserFormP => UsersUserFormPageTag, UserFormPage;
        pane UserDetailIdx, UserDetailP => UsersUserDetailPageTag, UserDetailPage;
        page ConfirmDeleteIdx, ConfirmDeleteP => UsersConfirmDeletePageTag, ConfirmDeletePage;
        page UserSelectIdx, UserSelectP => UsersUserSelectPageTag, UserSelectPage;
        pane RoleListIdx, RoleListP => UsersRoleListPageTag, RoleListPage;
        pane RoleFormIdx, RoleFormP => UsersRoleFormPageTag, RoleFormPage;
        page RoleCreateModalIdx, RoleCreateModalP => UsersRoleCreateModalPageTag, RoleCreateModalPage;
        pane RoleDetailIdx, RoleDetailP => UsersRoleDetailPageTag, RoleDetailPage;
        page RoleSelectIdx, RoleSelectP => UsersRoleSelectPageTag, RoleSelectPage;
    ];
    routes: [
        get UsersLoginGetRouteTag, "/users/login", handlers::auth::login_get;
        post UsersLoginPostRouteTag, "/users/login", handlers::auth::login_post;
        get UsersSignupGetRouteTag, "/users/signup", handlers::auth::signup_get;
        post UsersSignupPostRouteTag, "/users/signup", handlers::auth::signup_post;
        get UsersLogoutGetRouteTag, "/users/logout", bare handlers::auth::logout;
        post UsersLogoutPostRouteTag, "/users/logout", bare handlers::auth::logout;
        get UsersUnauthenticatedRouteTag, "/users/unauthenticated", handlers::auth::unauthenticated;
        get UsersLoginSuccessRouteTag, "/users/success", bare handlers::auth::login_success;
        get UsersSelfRouteTag, "/users/self", handlers::self_profile::detail;
        get UsersSelfEditGetRouteTag, "/users/self/edit", handlers::self_profile::edit_get;
        post UsersSelfEditPostRouteTag, "/users/self/edit", handlers::self_profile::edit_post;
        get UsersSelfChangePasswordGetRouteTag, "/users/self/change-password", handlers::self_profile::change_password_get;
        post UsersSelfChangePasswordPostRouteTag, "/users/self/change-password", handlers::self_profile::change_password_post;
        get UsersListRouteTag, "/users", handlers::users::list;
        get UsersSelectRouteTag, "/users/select", handlers::users::select;
        get UsersCreateGetRouteTag, "/users/create", handlers::users::create_get;
        post UsersCreatePostRouteTag, "/users/create", handlers::users::create_post;
        get UsersDetailRouteTag, "/users/u/{id}", handlers::users::detail;
        get UsersEditGetRouteTag, "/users/u/{id}/edit", handlers::users::edit_get;
        post UsersEditPostRouteTag, "/users/u/{id}/edit", handlers::users::edit_post;
        get UsersDeleteGetRouteTag, "/users/u/{id}/delete", handlers::users::delete_get;
        post UsersDeletePostRouteTag, "/users/u/{id}/delete", bare handlers::users::delete_post;
        get UsersChangePasswordGetRouteTag, "/users/u/{id}/change-password", handlers::users::change_password_get;
        post UsersChangePasswordPostRouteTag, "/users/u/{id}/change-password", handlers::users::change_password_post;
        get UsersRolesListRouteTag, "/users/roles", handlers::roles::list;
        get UsersRolesSelectRouteTag, "/users/roles/select", handlers::roles::select;
        get UsersRolesCreateGetRouteTag, "/users/roles/create", handlers::roles::create_get;
        post UsersRolesCreatePostRouteTag, "/users/roles/create", handlers::roles::create_post;
        get UsersRolesDetailRouteTag, "/users/roles/{id}", handlers::roles::detail;
        get UsersRolesEditGetRouteTag, "/users/roles/{id}/edit", handlers::roles::edit_get;
        post UsersRolesEditPostRouteTag, "/users/roles/{id}/edit", handlers::roles::edit_post;
        get UsersRolesDeleteGetRouteTag, "/users/roles/{id}/delete", handlers::roles::delete_get;
        post UsersRolesDeletePostRouteTag, "/users/roles/{id}/delete", bare handlers::roles::delete_post;
    ]
}
