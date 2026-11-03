use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, Crumb, FieldSubtitle, FieldText, FieldTitle, ShellChrome, SidebarMenu,
    SidebarMenuItem, button_modal_form, container_column, container_row, detail, field_subtitle,
    field_text, field_title, label, sidebar_menu, sidebar_menu_item_pane,
};
use lariv_core::template::RenderTemplate;

use crate::keys::{SELF_EDIT_MODAL, USER_EDIT_MODAL};
use crate::routes::{
    UsersChangePasswordGetRouteTag, UsersDetailRouteTag, UsersEditGetRouteTag,
    UsersEditPostRouteTag, UsersListRouteTag, UsersSelfEditGetRouteTag, UsersSelfEditPostRouteTag,
};

use super::self_::user_self_menu;
use super::{app_scaffold, scaffold_main, scaffold_pane, self_crumbs};

pub(crate) fn user_detail_menu(
    user_id: i64,
    user_name: &str,
    active: &str,
    show_change_password: bool,
) -> Markup {
    let title = format!("User: {user_name}");
    let detail_url = UsersDetailRouteTag::new(user_id).url();
    let pw_url = UsersChangePasswordGetRouteTag::new(user_id).url();
    sidebar_menu(SidebarMenu {
        title: &title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "User Detail",
                url: &detail_url,
                active: active == "detail",
                ..Default::default()
            }))
            @if show_change_password {
                (sidebar_menu_item_pane(SidebarMenuItem {
                    title: "Change Password",
                    url: &pw_url,
                    active: active == "password",
                    ..Default::default()
                }))
            }
        },
    })
}

pub(crate) fn user_crumbs(id: i64, name: &str, action: Option<&str>) -> Markup {
    use lariv_core::components::breadcrumbs;
    let list_url = UsersListRouteTag.url();
    let detail_url = UsersDetailRouteTag::new(id).url();
    match action {
        None => breadcrumbs(&[
            Crumb {
                label: "Users",
                href: Some(&list_url),
            },
            Crumb {
                label: "All Users",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: None,
            },
        ]),
        Some(act) => breadcrumbs(&[
            Crumb {
                label: "Users",
                href: Some(&list_url),
            },
            Crumb {
                label: "All Users",
                href: Some(&list_url),
            },
            Crumb {
                label: name,
                href: Some(&detail_url),
            },
            Crumb {
                label: act,
                href: None,
            },
        ]),
    }
}

#[derive(Generic)]
pub struct SelfDetailPage {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub timezone: String,
    pub role: String,
    /// Set only when the viewer is a superuser.
    pub last_login_time: Option<String>,
}

impl SelfDetailPage {
    fn pane_body(&self) -> Markup {
        let edit_get = UsersSelfEditGetRouteTag.url();
        let edit_post = UsersSelfEditPostRouteTag.path();
        detail(html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: &self.name,
                        classes: "",
                    }))
                    (field_subtitle(FieldSubtitle {
                        value: &self.email,
                        classes: "",
                    }))
                    (label(
                        "Phone",
                        field_text(FieldText {
                            value: &self.phone,
                            classes: "",
                        }),
                    ))
                    (label(
                        "Timezone",
                        field_text(FieldText {
                            value: &self.timezone,
                            classes: "",
                        }),
                    ))
                    (label(
                        "Role",
                        field_text(FieldText {
                            value: &self.role,
                            classes: "",
                        }),
                    ))
                    @if let Some(last_login_time) = &self.last_login_time {
                        (label(
                            "Last login",
                            field_text(FieldText {
                                value: last_login_time,
                                classes: "",
                            }),
                        ))
                    }
                    (container_row("flex gap-2 mt-4", html! {
                        (button_modal_form(ButtonModalForm {
                            name: "p_users.SelfEditForm",
                            href: &edit_get,
                            form_post_url: &edit_post,
                            modal_uid: SELF_EDIT_MODAL.id(),
                            label: "Edit",
                            classes: "btn-outline",
                            ..Default::default()
                        }))
                    }))
                },
            ))
        })
    }
}

impl lariv_core::template::RenderAppPane for SelfDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = self_crumbs(&self.name, None);
        scaffold_pane(
            user_self_menu(&self.name, "detail"),
            crumbs,
            self.pane_body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(self_crumbs(&self.name, None), self.pane_body())
    }
}

impl RenderTemplate for SelfDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = self_crumbs(&self.name, None);
        app_scaffold(
            "Profile — Lariv",
            chrome,
            user_self_menu(&self.name, "detail"),
            crumbs,
            self.pane_body(),
        )
    }
}

#[derive(Generic)]
pub struct UserDetailPage {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub timezone: String,
    pub role: String,
    pub show_change_password: bool,
    /// Set only when the viewer is a superuser.
    pub last_login_time: Option<String>,
}

impl UserDetailPage {
    fn pane_body(&self) -> Markup {
        let edit_get = UsersEditGetRouteTag::new(self.id).url();
        let edit_post = UsersEditPostRouteTag::new(self.id).path();
        detail(html! {
            (container_column(
                "",
                html! {
                    (field_title(FieldTitle {
                        value: &self.name,
                        classes: "",
                    }))
                    (field_subtitle(FieldSubtitle {
                        value: &self.email,
                        classes: "",
                    }))
                    (label(
                        "Phone",
                        field_text(FieldText {
                            value: &self.phone,
                            classes: "",
                        }),
                    ))
                    (label(
                        "Timezone",
                        field_text(FieldText {
                            value: &self.timezone,
                            classes: "",
                        }),
                    ))
                    (label(
                        "Role",
                        field_text(FieldText {
                            value: &self.role,
                            classes: "",
                        }),
                    ))
                    @if let Some(last_login_time) = &self.last_login_time {
                        (label(
                            "Last login",
                            field_text(FieldText {
                                value: last_login_time,
                                classes: "",
                            }),
                        ))
                    }
                    (container_row("flex gap-2 mt-4", html! {
                        (button_modal_form(ButtonModalForm {
                            name: "p_users.UserEditForm",
                            href: &edit_get,
                            form_post_url: &edit_post,
                            modal_uid: USER_EDIT_MODAL.id(),
                            label: "Edit",
                            classes: "btn-outline",
                            ..Default::default()
                        }))
                    }))
                },
            ))
        })
    }
}

impl lariv_core::template::RenderAppPane for UserDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        let crumbs = user_crumbs(self.id, &self.name, None);
        scaffold_pane(
            user_detail_menu(self.id, &self.name, "detail", self.show_change_password),
            crumbs,
            self.pane_body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(user_crumbs(self.id, &self.name, None), self.pane_body())
    }
}
impl RenderTemplate for UserDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        let crumbs = user_crumbs(self.id, &self.name, None);
        app_scaffold(
            &format!("{} — Lariv", self.name),
            chrome,
            user_detail_menu(self.id, &self.name, "detail", self.show_change_password),
            crumbs,
            self.pane_body(),
        )
    }
}
