use lariv_core::components::{sort_desc, sort_key};
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Select};

use super::entities::bank::{self, Entity as BankEntity};
use super::entities::bank_account::{self, Entity as BankAccountEntity};
use super::routes::BankAccountsView;

pub async fn find_bank_account_scoped(
    db: &DatabaseConnection,
    id: i64,
) -> Option<bank_account::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<BankAccountsView, _>(BankAccountEntity::find_by_id(id))
            .one(db)
            .await,
        "find bank account scoped",
    )
}

pub async fn find_bank_by_ifsc(db: &DatabaseConnection, ifsc: &str) -> Option<bank::Model> {
    let ifsc = ifsc.trim();
    if ifsc.is_empty() {
        return None;
    }
    lariv_core::web::opt_or_log(
        BankEntity::find()
            .filter(bank::Column::Ifsc.eq(ifsc))
            .one(db)
            .await,
        "find bank by ifsc",
    )
}

pub fn apply_bank_account_sort(
    query: Select<BankAccountEntity>,
    sort: Option<&str>,
) -> Select<BankAccountEntity> {
    let sort = sort.unwrap_or("").trim();
    match sort_key(sort) {
        s if s.eq_ignore_ascii_case("Name") => {
            if sort_desc(sort) {
                query.order_by_desc(bank_account::Column::Name)
            } else {
                query.order_by_asc(bank_account::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("IFSC") => {
            if sort_desc(sort) {
                query.order_by_desc(bank_account::Column::IfscCode)
            } else {
                query.order_by_asc(bank_account::Column::IfscCode)
            }
        }
        s if s.eq_ignore_ascii_case("Account Number") => {
            if sort_desc(sort) {
                query.order_by_desc(bank_account::Column::AccountNumber)
            } else {
                query.order_by_asc(bank_account::Column::AccountNumber)
            }
        }
        _ => query.order_by_desc(bank_account::Column::Id),
    }
}

pub fn apply_bank_sort(query: Select<BankEntity>, sort: Option<&str>) -> Select<BankEntity> {
    let sort = sort.unwrap_or("").trim();
    match sort_key(sort) {
        s if s.eq_ignore_ascii_case("IFSC") => {
            if sort_desc(sort) {
                query.order_by_desc(bank::Column::Ifsc)
            } else {
                query.order_by_asc(bank::Column::Ifsc)
            }
        }
        s if s.eq_ignore_ascii_case("Bank") => {
            if sort_desc(sort) {
                query.order_by_desc(bank::Column::BankName)
            } else {
                query.order_by_asc(bank::Column::BankName)
            }
        }
        s if s.eq_ignore_ascii_case("Branch") => {
            if sort_desc(sort) {
                query.order_by_desc(bank::Column::BranchName)
            } else {
                query.order_by_asc(bank::Column::BranchName)
            }
        }
        _ => query.order_by_asc(bank::Column::Ifsc),
    }
}

pub async fn bank_display_label(db: &DatabaseConnection, ifsc: &str) -> String {
    find_bank_by_ifsc(db, ifsc)
        .await
        .map(|b| b.display_label())
        .unwrap_or_else(|| ifsc.trim().to_string())
}
