use lariv_plugin_users::roles::{Admin, Unassigned};

use super::{
    handlers,
    keys::{COMPANY_DELETE_MODAL, COMPANY_SELECT_MODAL, COMPANY_SELECT_TABLE, COMPANY_TABLE},
};

pub struct CompaniesView;

pub struct CompaniesMutate;

pub struct CompaniesMembersMutate;

lariv_core::define_plugin_routes! {
    plugin: CompaniesTag;
    prefix: "/dashboard";
    routes: [
        get CompanyDefaultRouteTag, "/companies", handlers::companies::list, fragment(COMPANY_TABLE), authorize(CompaniesView, [Unassigned, Admin]);
        get CompanyCreateGetRouteTag, "/companies/create", handlers::companies::create_get, modal, authorize(CompaniesMutate, [Unassigned, Admin]);
        post CompanyCreatePostRouteTag, "/companies/create", handlers::companies::create_post, authorize(CompaniesMutate, [Unassigned, Admin]);
        get CompanyDetailRouteTag, "/companies/{id}", handlers::companies::detail, authorize(CompaniesView, [Unassigned, Admin]);
        get CompanyEditGetRouteTag, "/companies/{id}/edit", handlers::companies::edit_get, modal, authorize(CompaniesMutate, [Unassigned, Admin]);
        post CompanyEditPostRouteTag, "/companies/{id}/edit", handlers::companies::edit_post, authorize(CompaniesMutate, [Unassigned, Admin]);
        get CompanyDeleteGetRouteTag, "/companies/{id}/delete", handlers::companies::delete_get, modal, authorize(CompaniesMutate, [Unassigned, Admin]);
        post CompanyDeletePostRouteTag, "/companies/{id}/delete", bare handlers::companies::delete_post, fragment(COMPANY_DELETE_MODAL), authorize(CompaniesMutate, [Unassigned, Admin]);
        get CompanyFkSelectRouteTag, "/companies/pick", handlers::companies::select, fk_select(COMPANY_SELECT_TABLE, COMPANY_SELECT_MODAL), authorize(CompaniesView, [Unassigned, Admin]);
        post CompanyMemberAddPostRouteTag, "/companies/{id}/members", handlers::companies::member_add_post, authorize(CompaniesMembersMutate, [Unassigned, Admin]);
        post CompanyMemberRemovePostRouteTag, "/companies/{id}/members/{membership_id}/remove", bare handlers::companies::member_remove_post, redirect, authorize(CompaniesMembersMutate, [Unassigned, Admin]);
    ]
}
