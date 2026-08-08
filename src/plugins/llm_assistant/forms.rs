//! Request form structs for the LLM assistant plugin.

use crate::html_form::{
    html_form,
    widgets::{ManyToMany, Text, Textarea},
};

#[html_form]
pub struct PreferencesForm {
    #[form(label = "Gemini API key", widget = Text)]
    pub api_key: String,
}

#[html_form]
pub struct SkillForm {
    #[form(label = "Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Description", widget = Text)]
    pub description: String,

    #[form(label = "Content", required, widget = Textarea, rows = 12)]
    pub content: String,

    #[form(
        label = "Files",
        widget = ManyToMany,
        url = "/filesystem/file-select/",
        swap_key = "fk-llm-skill-files",
        placeholder = "Select files..."
    )]
    pub files: Vec<i64>,
}

#[html_form]
pub struct SkillNameFilterForm {
    #[form(label = "Name", widget = Text)]
    pub name: String,
}
