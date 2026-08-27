use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Select};

use crate::plugins::users::state::AuthContext;

use crate::plugins::finance_common::is_superuser;

use crate::plugins::finance_creditnotes::entities::credit_note::{
    self, Entity as CreditNoteEntity,
};

pub fn scope_credit_notes(
    query: Select<CreditNoteEntity>,
    auth: &AuthContext,
) -> Select<CreditNoteEntity> {
    if is_superuser(auth) {
        return query;
    }
    query.filter(credit_note::Column::Id.eq(-1))
}

pub async fn find_credit_note_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<credit_note::Model> {
    let query = CreditNoteEntity::find_by_id(id);
    crate::web::opt_or_log(
        scope_credit_notes(query, auth).one(db).await,
        "find credit note scoped",
    )
}

pub fn order_credit_notes(query: Select<CreditNoteEntity>) -> Select<CreditNoteEntity> {
    query
        .order_by_desc(credit_note::Column::Datetime)
        .order_by_desc(credit_note::Column::Id)
}
