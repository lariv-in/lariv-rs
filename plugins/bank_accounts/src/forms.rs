use lariv_core::html_form::{html_form, widgets::Text};

use crate::routes::{BankFkSelectRouteTag, BankMicrSelectRouteTag};

#[html_form]
pub struct BankAccountForm {
    #[form(label = "Bank Account Name", required, widget = Text)]
    pub name: String,

    #[form(label = "Bank Account Number", required, widget = Text)]
    pub account_number: String,

    #[form(
        label = "Bank Account IFSC Code",
        required,
        widget = ForeignKey,
        route = BankFkSelectRouteTag,
        swap_key = "bank-account-ifsc",
        display = "ifsc",
        placeholder = "Select IFSC…"
    )]
    pub ifsc_code: String,

    #[form(
        label = "Bank Account MICR Code",
        widget = ForeignKey,
        route = BankMicrSelectRouteTag,
        swap_key = "bank-account-micr",
        placeholder = "Select MICR…"
    )]
    pub micr_code: String,
}
