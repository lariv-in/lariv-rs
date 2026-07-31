//! Request form structs for blog admin.

use crate::html_form::{
    html_form,
    widgets::{ForeignKey, ManyToMany, Text, Textarea},
};

#[html_form]
pub struct BlogForm {
    #[form(label = "Title", required, widget = Text)]
    pub title: String,

    #[form(label = "Slug", widget = Text)]
    pub slug: String,

    #[form(label = "Description", widget = Textarea, rows = 3)]
    pub description: String,

    #[form(
        label = "Author",
        widget = ForeignKey,
        url = "/users/select/",
        swap_key = "fk-blog-author",
        display = "author",
        required,
        placeholder = "Select an author..."
    )]
    pub created_by_id: i64,

    #[form(
        label = "Tags",
        widget = ManyToMany,
        url = "/blog/tags/select/",
        swap_key = "fk-blog-tags",
        placeholder = "Select tags..."
    )]
    pub tags: Vec<i64>,

    #[form(label = "Content", widget = Textarea, rows = 12)]
    pub content: String,
}

#[html_form]
pub struct TagForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,
}

#[html_form]
pub struct BlogTitleFilterForm {
    #[form(label = "Title", widget = Text)]
    pub title: String,
}

#[html_form]
pub struct TagNameFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,
}
