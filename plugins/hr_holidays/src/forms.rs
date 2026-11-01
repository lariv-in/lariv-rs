use lariv_core::html_form::{
    html_form,
    widgets::{Date, Number, Text, Textarea},
};

#[html_form]
pub struct HolidayForm {
    #[form(label = "Title", required, widget = Text)]
    pub title: String,

    #[form(label = "Description", widget = Textarea, rows = 4)]
    pub description: String,

    #[form(label = "Date", required, widget = Date)]
    pub date: String,
}

#[html_form]
pub struct HolidayImportForm {
    #[form(label = "Year", required, widget = Number)]
    pub year: String,
}
