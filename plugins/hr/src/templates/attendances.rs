use frunk::Generic;
use maud::{Markup, PreEscaped, html};

use lariv_core::components::{
    ButtonModalForm, ButtonSubmit, DeleteConfirmation, DetailHeader, FieldDatetime, FieldText,
    FilterClauseView, FormOpts, ListFilterBuilder, ObjectList, PaginationPage, ShellChrome,
    SwapKey, TableButtonFilter, TableColumnHeader, TablePagination, TableRow, button_modal_form,
    button_submit, column_sort_url, container_column, delete_confirmation, detail, detail_header,
    field_datetime, field_text, form, form_hx_get_route, form_hx_post_route, form_hx_post_selector,
    form_hx_post_url, icon, label, list_filter_builder, modal, modal_keyed, pagination_pages,
    row_attr_navigate, sort_indicator, table_button_filter, table_pagination,
};
use lariv_core::html_form::{CsrfToken, FormCtx, HtmlForm};
use lariv_core::list_filters::{FilterField, FilterRowError};
use lariv_core::template::{RenderAppPane, RenderTemplate};
use lariv_core::web::modal_create_post_url;

use crate::{
    crumbs::{attendance_crumbs, attendances_list_crumbs},
    forms::{AttendanceForm, AttendanceFormField},
    keys::{
        AttendanceCreateModalKey, AttendanceDeleteModalKey, AttendanceEditModalKey,
        AttendanceTableKey,
    },
    routes::{
        AttendanceCreateGetRouteTag, AttendanceCreatePostRouteTag, AttendanceDeleteGetRouteTag,
        AttendanceDeletePostRouteTag, AttendanceEditGetRouteTag, AttendanceEditPostRouteTag,
        AttendanceListRouteTag, AttendancePunchInRouteTag, AttendancePunchOutRouteTag,
    },
    templates::{app_scaffold, hr_menu, scaffold_main, scaffold_pane},
};

#[derive(Clone)]
pub struct AttendanceRow {
    pub id: i64,
    pub user: String,
    pub started_at: String,
    pub ended_at: String,
    pub detail_href: String,
}

fn punch_button<K: SwapKey, R: lariv_core::http::RouteUrl + lariv_core::http::FragmentPost<K>>(
    route: R,
    label: &str,
    title: &str,
    icon_name: &str,
    classes: &str,
    enabled: bool,
) -> Markup {
    let class = format!("btn btn-lg min-w-44 gap-2 {classes}");
    // Swap the attendance panel (buttons plus table), not the table toolbar.
    let attrs = form_hx_post_route::<K, R>(route)
        .set("hx-target", "closest .hr-attendance")
        .set("hx-select", ".hr-attendance")
        .set("hx-swap", "outerHTML");
    html! {
        (PreEscaped(format!(r#"<form class="inline-flex" method="POST"{}>"#, attrs.as_string())))
        button type="submit" class=(class) title=(title) aria-label=(label) disabled[!enabled] {
            (icon(icon_name, "size-6"))
            span { (label) }
        }
        (PreEscaped("</form>"))
    }
}

fn render_pagination<K: SwapKey>(path_and_query: &str, number: u32, num_pages: u32) -> Markup {
    let owned = pagination_pages(path_and_query, number, num_pages, true);
    let pages: Vec<PaginationPage<'_>> = owned
        .iter()
        .map(|(ellipsis, url, push_url, active, label)| PaginationPage {
            ellipsis: *ellipsis,
            url: url.as_str(),
            push_url: *push_url,
            active: *active,
            label: label.as_str(),
        })
        .collect();
    table_pagination(TablePagination {
        pages: &pages,
        hx_target: K::SELECTOR,
    })
}

fn fk_value(id: i64) -> String {
    if id <= 0 {
        String::new()
    } else {
        id.to_string()
    }
}

fn attendance_form_inputs(
    user_id: i64,
    user_display: &str,
    started_at: &str,
    ended_at: &str,
) -> Markup {
    let user_id = fk_value(user_id);
    AttendanceForm::render_inputs(
        &FormCtx::form::<AttendanceForm>(CsrfToken::current())
            .value(AttendanceFormField::UserId, user_id.as_str())
            .display(AttendanceFormField::UserId, user_display)
            .value(AttendanceFormField::StartedAt, started_at)
            .value(AttendanceFormField::EndedAt, ended_at),
    )
}

#[derive(Clone)]
pub struct AttendancePunchBar {
    pub punch_in_enabled: bool,
    pub punch_out_enabled: bool,
    pub note: String,
}

#[derive(Generic)]
pub struct AttendanceListPage {
    pub rows: ObjectList<AttendanceRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub path_and_query: String,
    pub page_size: u32,
    pub error: String,
    pub punch: Option<AttendancePunchBar>,
}

impl AttendanceListPage {
    pub fn render_table(&self) -> Markup {
        let user_sort = column_sort_url(&self.path_and_query, "User", &self.sort);
        let start_sort = column_sort_url(&self.path_and_query, "StartedAt", &self.sort);
        let end_sort = column_sort_url(&self.path_and_query, "EndedAt", &self.sort);
        let user_label = format!("User{}", sort_indicator(&self.sort, "User"));
        let start_label = format!("Start{}", sort_indicator(&self.sort, "StartedAt"));
        let end_label = format!("End{}", sort_indicator(&self.sort, "EndedAt"));
        let headers = [
            TableColumnHeader {
                key: "User",
                label: &user_label,
                sort_url: Some(&user_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "StartedAt",
                label: &start_label,
                sort_url: Some(&start_sort),
                push_url: true,
            },
            TableColumnHeader {
                key: "EndedAt",
                label: &end_label,
                sort_url: Some(&end_sort),
                push_url: true,
            },
        ];
        let table_rows: Vec<TableRow> = self
            .rows
            .items
            .iter()
            .map(|row| TableRow {
                attrs: row_attr_navigate(&row.detail_href),
                cells: vec![
                    field_text(FieldText {
                        value: &row.user,
                        classes: "",
                    }),
                    field_datetime(FieldDatetime {
                        value: &row.started_at,
                        classes: "",
                    }),
                    field_datetime(FieldDatetime {
                        value: &row.ended_at,
                        classes: "",
                    }),
                ],
            })
            .collect();
        let mut actions = html! {
            (table_button_filter(TableButtonFilter {
                panel: list_filter_builder(ListFilterBuilder {
                    fields: &self.fields,
                    rows: &self.filters,
                    page_size: self.page_size,
                    sort: &self.sort,
                    attrs: form_hx_get_route::<AttendanceTableKey, AttendanceListRouteTag>(
                        AttendanceListRouteTag,
                    ),
                    apply_label: "Apply",
                    extra: html! {},
                }),
                ..Default::default()
            }))
        };
        if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<
                super::super::routes::AttendanceMutate,
            >(),
        ) {
            actions = html! {
                (actions)
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.AttendanceCreateForm",
                    href: &AttendanceCreateGetRouteTag.url(),
                    form_post_url: &AttendanceCreateGetRouteTag.path(),
                    modal_uid: AttendanceCreateModalKey::ID,
                    icon_name: Some("plus"),
                    classes: "btn-square btn-outline btn-sm",
                    ..Default::default()
                }))
            };
        }
        let punch_bar = if let Some(punch) = &self.punch {
            let note = punch.note.as_str();
            let show_note = note != self.error.as_str();
            let punch_out_title = if punch.punch_out_enabled {
                "Punch out"
            } else {
                "No open attendance."
            };
            html! {
                div class="mb-4 flex flex-col gap-3 rounded-box border border-base-300 bg-base-200 px-4 py-4" {
                    @if !self.error.is_empty() {
                        div role="alert" class="alert alert-error py-2 text-sm" { (self.error) }
                    }
                    div class="flex flex-wrap items-center gap-4" {
                        (punch_button::<AttendanceTableKey, AttendancePunchInRouteTag>(
                            AttendancePunchInRouteTag,
                            "Punch in",
                            note,
                            "arrow-right-end-on-rectangle",
                            "btn-success",
                            punch.punch_in_enabled,
                        ))
                        (punch_button::<AttendanceTableKey, AttendancePunchOutRouteTag>(
                            AttendancePunchOutRouteTag,
                            "Punch out",
                            punch_out_title,
                            "arrow-right-start-on-rectangle",
                            "btn-warning",
                            punch.punch_out_enabled,
                        ))
                        @if show_note {
                            p class="text-sm" { (note) }
                        }
                    }
                }
            }
        } else if !self.error.is_empty() {
            let message = self.error.as_str();
            html! {
                div role="alert" class="alert alert-error py-2 text-sm mb-2" { (message) }
            }
        } else {
            html! {}
        };
        let table = super::data_table_with_filter_errors::<AttendanceTableKey>(
            "Attendance",
            actions,
            &headers,
            &table_rows,
            render_pagination::<AttendanceTableKey>(
                &self.path_and_query,
                self.rows.number,
                self.rows.num_pages,
            ),
            &self.path_and_query,
            &self.filter_errors,
        );
        html! {
            div class="hr-attendance" {
                (punch_bar)
                (table)
            }
        }
    }
}

impl RenderTemplate for AttendanceListPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Attendance — Lariv",
            chrome,
            hr_menu("attendance"),
            attendances_list_crumbs(),
            self.render_table(),
        )
    }
}

impl RenderAppPane for AttendanceListPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            hr_menu("attendance"),
            attendances_list_crumbs(),
            self.render_table(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(attendances_list_crumbs(), self.render_table())
    }
}

#[derive(Generic)]
pub struct AttendanceDetailPage {
    pub id: i64,
    pub user: String,
    pub started_at: String,
    pub ended_at: String,
}

impl AttendanceDetailPage {
    fn body(&self) -> Markup {
        let actions = if lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<
                super::super::routes::AttendanceMutate,
            >(),
        ) {
            html! {
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.AttendanceEditForm",
                    href: &AttendanceEditGetRouteTag::new(self.id).url(),
                    form_post_url: &AttendanceEditPostRouteTag::new(self.id).path(),
                    modal_uid: AttendanceEditModalKey::ID,
                    label: "Edit",
                    classes: "btn-outline btn-sm",
                    ..Default::default()
                }))
                (button_modal_form(ButtonModalForm {
                    name: "p_hr.AttendanceDeleteForm",
                    href: &AttendanceDeleteGetRouteTag::new(self.id).url(),
                    form_post_url: &AttendanceDeleteGetRouteTag::new(self.id).path(),
                    modal_uid: AttendanceDeleteModalKey::ID,
                    label: "Delete",
                    classes: "btn-outline btn-error btn-sm",
                    ..Default::default()
                }))
            }
        } else {
            html! {}
        };
        html! {
            (detail(html! {
                (container_column("", html! {
                    (detail_header(DetailHeader {
                        title: &self.user,
                        actions,
                    }))
                    (label("User", field_text(FieldText { value: &self.user, classes: "" })))
                    (label("Start", field_datetime(FieldDatetime { value: &self.started_at, classes: "" })))
                    (label("End", field_datetime(FieldDatetime { value: &self.ended_at, classes: "" })))
                }))
            }))
        }
    }
}

impl RenderTemplate for AttendanceDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            "Attendance — Lariv",
            chrome,
            hr_menu("attendance"),
            attendance_crumbs(&self.user),
            self.body(),
        )
    }
}

impl RenderAppPane for AttendanceDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            hr_menu("attendance"),
            attendance_crumbs(&self.user),
            self.body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(attendance_crumbs(&self.user), self.body())
    }
}

pub struct AttendanceCreateModalPage {
    pub form_name: String,
    pub refresh_table: String,
    pub user_id: i64,
    pub user_display: String,
    pub started_at: String,
    pub ended_at: String,
    pub error: String,
}

impl AttendanceCreateModalPage {
    pub fn new(form_name: String, refresh_table: String) -> Self {
        Self {
            form_name,
            refresh_table,
            user_id: 0,
            user_display: String::new(),
            started_at: String::new(),
            ended_at: String::new(),
            error: String::new(),
        }
    }

    pub fn with_form(
        form_name: String,
        refresh_table: String,
        form: &AttendanceForm,
        user_display: String,
        error: String,
    ) -> Self {
        Self {
            form_name,
            refresh_table,
            user_id: form.user_id,
            user_display,
            started_at: form.started_at.clone(),
            ended_at: form.ended_at.clone(),
            error,
        }
    }
}

impl RenderTemplate for AttendanceCreateModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<AttendanceCreateModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "New attendance" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<AttendanceCreateModalKey>(&modal_create_post_url(
                        AttendanceCreatePostRouteTag,
                        &self.form_name,
                        &self.refresh_table,
                    )),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: attendance_form_inputs(
                        self.user_id,
                        &self.user_display,
                        &self.started_at,
                        &self.ended_at,
                    ),
                    actions: html! {
                        (button_submit(ButtonSubmit { label: "Create", ..Default::default() }))
                    },
                    ..Default::default()
                }))
            },
        )
    }
}

pub struct AttendanceEditModalPage {
    pub id: i64,
    pub form_name: String,
    pub post_url: String,
    pub user_id: i64,
    pub user_display: String,
    pub started_at: String,
    pub ended_at: String,
    pub error: String,
}

impl RenderTemplate for AttendanceEditModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        modal_keyed::<AttendanceEditModalKey>(
            &self.form_name,
            html! {
                h3 class="font-bold text-lg mb-4" { "Edit attendance" }
                (form(&CsrfToken::current(), FormOpts {
                    attrs: form_hx_post_url::<AttendanceEditModalKey>(&self.post_url),
                    form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                    inputs: attendance_form_inputs(
                        self.user_id,
                        &self.user_display,
                        &self.started_at,
                        &self.ended_at,
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

pub struct AttendanceDeleteModalPage {
    pub id: i64,
    pub form_name: String,
    pub message: String,
    pub error: String,
}

impl RenderTemplate for AttendanceDeleteModalPage {
    fn render(&self, _chrome: &ShellChrome) -> Markup {
        let target = format!("#{}", AttendanceDeleteModalKey::ID);
        let post_url = AttendanceDeletePostRouteTag::new(self.id).url();
        modal(lariv_core::components::Modal {
            uid: AttendanceDeleteModalKey::ID,
            children: delete_confirmation(DeleteConfirmation {
                title: "Confirm deletion",
                message: &self.message,
                attrs: form_hx_post_selector(&post_url, &target),
                form_error: Some(self.error.as_str()).filter(|e| !e.is_empty()),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{AttendanceListPage, AttendancePunchBar};
    use lariv_core::components::ObjectList;

    fn render(punch: Option<AttendancePunchBar>) -> String {
        AttendanceListPage {
            rows: ObjectList::from_page(vec![], 1, 20, 0),
            fields: Vec::new(),
            filters: Vec::new(),
            filter_errors: Vec::new(),
            sort: String::new(),
            path_and_query: "/dashboard/hr/attendances".into(),
            page_size: 20,
            error: String::new(),
            punch,
        }
        .render_table()
        .into_string()
    }

    fn submit_is_disabled(html: &str, label: &str) -> bool {
        let marker = format!("aria-label=\"{label}\"");
        let at = html
            .find(&marker)
            .unwrap_or_else(|| panic!("missing {label}"));
        let tag_start = html[..at].rfind("<button").expect("button tag");
        let tag_end = html[at..].find('>').expect("tag end") + at;
        html[tag_start..tag_end].contains("disabled")
    }

    #[test]
    fn punch_buttons_are_large_and_above_the_table() {
        let html = render(Some(AttendancePunchBar {
            punch_in_enabled: false,
            punch_out_enabled: true,
            note: "Already punched in.".into(),
        }));
        let punch_at = html.find("Punch in").expect("punch in");
        let table_at = html.find("<table").expect("table");
        assert!(punch_at < table_at, "{html}");
        assert!(html.contains("btn-lg"));
        assert!(html.contains("btn-success"));
        assert!(html.contains("btn-warning"));
        assert!(submit_is_disabled(&html, "Punch in"));
        assert!(!submit_is_disabled(&html, "Punch out"));
        assert!(html.contains("Already punched in."));
    }

    #[test]
    fn punch_buttons_are_hidden_when_the_viewer_cannot_punch() {
        let html = render(None);
        assert!(!html.contains("Punch in"));
        assert!(!html.contains("Punch out"));
    }
}
