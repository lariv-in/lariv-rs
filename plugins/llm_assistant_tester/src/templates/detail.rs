use frunk::Generic;
use maud::{Markup, html};

use lariv_core::components::{
    ButtonModalForm, Crumb, DetailHeader, FieldMarkdown, FieldText, FieldTextarea, ShellChrome,
    SidebarMenu, SidebarMenuItem, breadcrumbs, button_modal_form, container_column, detail,
    detail_header, field_markdown, field_text, field_textarea, label, label_hint, sidebar_menu,
    sidebar_menu_item_pane,
};
use lariv_core::template::RenderTemplate;

use crate::forms::{DEFINITION_HINT, SOLUTION_HINT, TEST_CASE_EDIT_FORM};
use crate::keys::TEST_CASE_EDIT_MODAL;
use crate::routes::{
    TestCaseDetailRouteTag, TestCaseEditGetRouteTag, TestCaseEditPostRouteTag, TestCaseListRouteTag,
};

use super::{app_scaffold, scaffold_main, scaffold_pane};

fn detail_crumbs(name: &str) -> Markup {
    let list_url = TestCaseListRouteTag.url();
    breadcrumbs(&[
        Crumb {
            label: "Assistant Tester",
            href: Some(&list_url),
        },
        Crumb {
            label: "Test Cases",
            href: Some(&list_url),
        },
        Crumb {
            label: name,
            href: None,
        },
    ])
}

fn detail_menu(id: i64, name: &str) -> Markup {
    let menu_title = format!("Test Case: {name}");
    let detail_url = TestCaseDetailRouteTag::new(id).url();
    sidebar_menu(SidebarMenu {
        title: &menu_title,
        children: html! {
            (sidebar_menu_item_pane(SidebarMenuItem {
                title: "Test Case",
                url: &detail_url,
                active: true,
                ..Default::default()
            }))
        },
    })
}

fn solution_body(solution: &str) -> Markup {
    if solution.is_empty() {
        return field_text(FieldText {
            value: "No solution script yet.",
            classes: "text-base-content/60",
        });
    }
    field_textarea(FieldTextarea {
        value: solution,
        classes: "font-mono text-sm border border-base-300 p-3 rounded-md overflow-x-auto",
    })
}

fn definition_body(definition: &str) -> Markup {
    if definition.is_empty() {
        field_text(FieldText {
            value: "No definition yet.",
            classes: "text-base-content/60",
        })
    } else {
        field_markdown(FieldMarkdown {
            value: definition,
            classes: "",
        })
    }
}

#[derive(Generic)]
pub struct TestCaseDetailPage {
    pub id: i64,
    pub name: String,
    pub definition: String,
    pub solution: String,
    pub author_name: String,
    pub updated_at: String,
}

impl TestCaseDetailPage {
    fn pane_body(&self) -> Markup {
        let edit_get = TestCaseEditGetRouteTag::new(self.id).url();
        let edit_post = TestCaseEditPostRouteTag::new(self.id).path();
        detail(html! {
            (container_column(
                "",
                html! {
                    (detail_header(DetailHeader {
                        title: &self.name,
                        actions: button_modal_form(ButtonModalForm {
                            name: TEST_CASE_EDIT_FORM,
                            href: &edit_get,
                            form_post_url: &edit_post,
                            modal_uid: TEST_CASE_EDIT_MODAL.id(),
                            label: "Edit",
                            classes: "btn-outline",
                            ..Default::default()
                        }),
                    }))
                    (label("Author", field_text(FieldText {
                        value: &self.author_name,
                        classes: "",
                    })))
                    (label("Updated", field_text(FieldText {
                        value: &self.updated_at,
                        classes: "",
                    })))
                    (label_hint(
                        "Test Case Definition",
                        Some(DEFINITION_HINT),
                        definition_body(&self.definition),
                    ))
                    (label_hint(
                        "Test Case Solution",
                        Some(SOLUTION_HINT),
                        solution_body(&self.solution),
                    ))
                },
            ))
        })
    }
}

impl lariv_core::template::RenderAppPane for TestCaseDetailPage {
    fn render_pane(&self) -> lariv_core::components::AppLayoutHtml {
        scaffold_pane(
            detail_menu(self.id, &self.name),
            detail_crumbs(&self.name),
            self.pane_body(),
        )
    }
    fn render_main(&self) -> lariv_core::components::MainContentHtml {
        scaffold_main(detail_crumbs(&self.name), self.pane_body())
    }
}

impl RenderTemplate for TestCaseDetailPage {
    fn render(&self, chrome: &ShellChrome) -> Markup {
        app_scaffold(
            chrome,
            detail_menu(self.id, &self.name),
            detail_crumbs(&self.name),
            self.pane_body(),
        )
    }
}
