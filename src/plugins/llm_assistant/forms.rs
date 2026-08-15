//! Request form structs for the LLM assistant plugin.

use crate::html_form::{
    Upload, html_form,
    widgets::{File, ManyToMany, Select, Text, Textarea},
};

#[html_form]
pub struct PreferencesForm {
    #[form(label = "Gemini API key", widget = Text)]
    pub api_key: String,

    #[form(label = "Gemini model", widget = Select, required, choices = "chat_model")]
    pub chat_model: String,
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

#[html_form(default)]
pub struct SkillImportForm {
    #[form(label = "Skill Zip File", widget = File, accept = ".zip", required)]
    pub file: Upload,
}
