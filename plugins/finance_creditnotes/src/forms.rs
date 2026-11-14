use lariv_core::html_form::{html_form, widgets::Text};

const CREDIT_NOTE_NUMBER_FORMAT_HINT: &str = "\
Applied when a credit note is created.\n\
\n\
Only these placeholders are replaced:\n\
• {{FISCAL_CODE}} — Indian fiscal year code (Apr–Mar, e.g. 24-25)\n\
• {{YY}} — two-digit year\n\
• {{YYYY}} — four-digit year\n\
• {{POSTED_SEQ}} — next credit note id (MAX(id)+1)\n\
• {{FISCAL_POSTED_SEQ}} — count of credit notes in the same Indian fiscal year, plus one\n\
\n\
Leave blank to default to CN/{{FISCAL_CODE}}/{{POSTED_SEQ}}.";

#[html_form]
pub struct CreditNotePreferencesForm {
    #[form(
        label = "Credit note number format",
        widget = Text,
        hint = CREDIT_NOTE_NUMBER_FORMAT_HINT
    )]
    pub credit_note_number_format: String,
}
