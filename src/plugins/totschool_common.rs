//! Shared helpers for Totschool plugins (roles, auth scoping).

pub mod client_detail_menu;
pub mod clients_menu;
pub mod doc_export;
pub mod gen_poll;
pub mod schema;
pub mod user_select;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Select};

use lariv_rs::plugins::users::state::AuthContext;

pub const ROLE_STUDENT: &str = "totschool_student";
pub const ROLE_ADMIN: &str = "totschool_admin";

/// Whether the user can see all records (admin or superuser).
pub fn is_admin(auth: &AuthContext) -> bool {
    auth.user.is_superuser || auth.role == ROLE_ADMIN
}

pub fn whatsapp_url(phone: &str, text: &str) -> String {
    let phone = phone
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>();
    let encoded = url_encode(text);
    format!("https://wa.me/{phone}?text={encoded}")
}

fn url_encode(input: &str) -> String {
    let mut out = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Apply created-by scoping for non-admin users on a SeaORM select.
pub fn scope_created_by<E>(
    query: Select<E>,
    auth: &AuthContext,
    column: E::Column,
) -> Select<E>
where
    E: EntityTrait,
    E::Column: ColumnTrait,
{
    if is_admin(auth) {
        query
    } else {
        query.filter(column.eq(auth.user.id))
    }
}
