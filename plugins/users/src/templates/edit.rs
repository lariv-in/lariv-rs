use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DeleteConfirmation, FormOpts, ShellChrome, button_modal_form,
    button_submit, delete_confirmation, form, form_hx_post_selector, form_hx_post_url, modal,
    modal_keyed,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::template::RenderTemplate;
use lariv_core::web::modal_edit_post_url;

use crate::forms::{SelfEditForm, SelfEditFormField, UserForm, UserFormField};
use crate::keys::{SELF_EDIT_MODAL, USER_DELETE_MODAL, USER_EDIT_MODAL};
use crate::routes::{
    UsersDeleteGetRouteTag, UsersDeletePostRouteTag, UsersEditPostRouteTag,
    UsersSelfEditPostRouteTag,
};

#[derive(Generic)]
pub struct SelfEditModalPage {
    pub form_name: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub timezone: String,
    pub error: String,
}

impl RenderTemplate for SelfEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed(
            SELF_EDIT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit my profile" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(SELF_EDIT_MODAL, &modal_edit_post_url(
                        UsersSelfEditPostRouteTag,
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: SelfEditForm::render_inputs(
                        &FormCtx::form::<SelfEditForm>(CsrfToken::current())
                            .value(SelfEditFormField::Name, self.name.as_str())
                            .value(SelfEditFormField::Email, self.email.as_str())
                            .value(SelfEditFormField::Phone, self.phone.as_str())
                            .value(SelfEditFormField::Timezone, self.timezone.as_str())
                            .choices(
                                SelfEditFormField::Timezone,
                                lariv_core::datetime::timezone_choices(),
                            ),
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

/// Edit user modal. Create uses [`UserCreateModalPage`].
#[derive(Generic)]
pub struct UserEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub timezone: String,
    pub role: String,
    pub error: String,
}

impl RenderTemplate for UserEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let delete_url = UsersDeleteGetRouteTag::new(self.id).url();
        let ctx = FormCtx::form::<UserForm>(CsrfToken::current())
            .value(UserFormField::Name, self.name.as_str())
            .value(UserFormField::Email, self.email.as_str())
            .value(UserFormField::Phone, self.phone.as_str())
            .value(UserFormField::Timezone, self.timezone.as_str())
            .choices(
                UserFormField::Timezone,
                lariv_core::datetime::timezone_choices(),
            )
            .value(UserFormField::Role, self.role.as_str());
        modal_keyed(
            USER_EDIT_MODAL,
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit user" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url(USER_EDIT_MODAL, &modal_edit_post_url(
                        UsersEditPostRouteTag::new(self.id),
                        &self.form_name,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: UserForm::render_inputs(&ctx),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Save", ..Default::default() }))
                        (button_modal_form(ButtonModalForm {
                            label: "Delete",
                            icon_name: Some("trash"),
                            name: "p_users.UserDeleteForm",
                            href: &delete_url,
                            form_post_url: &delete_url,
                            modal_uid: USER_DELETE_MODAL.id(),
                            classes: "btn-error",
                            ..Default::default()
                        }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

#[derive(Generic)]
pub struct ConfirmDeletePage {
    pub modal_uid: String,
    pub message: String,
    pub form_name: String,
    pub id: i64,
    pub error: String,
}

impl RenderTemplate for ConfirmDeletePage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = if self.modal_uid.is_empty() {
            format!("#{}", USER_DELETE_MODAL.id())
        } else {
            format!("#{}", self.modal_uid)
        };
        let uid = if self.modal_uid.is_empty() {
            USER_DELETE_MODAL.id()
        } else {
            self.modal_uid.as_str()
        };
        let post_url = UsersDeletePostRouteTag::new(self.id).url();
        modal(lariv_core::components::Modal {
            uid,
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm Deletion",
                message: &self.message,
                attrs: form_hx_post_selector(&post_url, &target),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}
