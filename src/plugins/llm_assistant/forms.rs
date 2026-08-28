//! Request form structs for the LLM assistant plugin.

use crate::html_form::{
    Upload, html_form,
    widgets::{Email, File, ForeignKey, ManyToMany, Password, Section, Select, Text, Textarea},
};

#[html_form]
pub struct PreferencesForm {
    #[form(label = "Gemini API key", widget = Text)]
    pub api_key: String,

    #[form(label = "Gemini model", widget = Select, required, choices = "chat_model")]
    pub chat_model: String,

    #[form(label = "Google CSE API key", widget = Text)]
    pub cse_api_key: String,

    #[form(label = "Google CSE CX", widget = Text)]
    pub cse_cx: String,

    #[form(widget = Section, label = "Email Settings")]
    _section_email: (),

    #[form(label = "IMAP Server", widget = Text, row = "imap")]
    pub imap_server: String,

    #[form(label = "IMAP Port", widget = Text, row = "imap")]
    pub imap_port: String,

    #[form(label = "SMTP Server", widget = Text, row = "smtp")]
    pub smtp_server: String,

    #[form(label = "SMTP Port", widget = Text, row = "smtp")]
    pub smtp_port: String,

    #[form(label = "Email", widget = Email)]
    pub email: String,

    #[form(label = "Password", widget = Password)]
    pub password: String,

    #[form(label = "Encryption", widget = Select, choices = "mail_encryption")]
    pub mail_encryption: String,

    #[form(label = "Email Filter", widget = Textarea, rows = 8)]
    pub email_filter: String,

    #[form(
        label = "Session owner",
        widget = ForeignKey,
        url = "/users/select/",
        swap_key = "fk-llm-email-owner",
        display = "email_owner",
        placeholder = "Select a user..."
    )]
    pub email_owner_user_id: Option<i64>,

    #[form(
        label = "Email attachments folder",
        widget = ForeignKey,
        url = "/filesystem/select",
        swap_key = "fk-llm-email-attachments",
        display = "email_attachments_parent",
        placeholder = "Select a folder..."
    )]
    pub email_attachments_parent_id: Option<i64>,
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
