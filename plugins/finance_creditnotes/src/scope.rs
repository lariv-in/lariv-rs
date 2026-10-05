use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder, Select};

use crate::entities::credit_note::{
    self, Entity as CreditNoteEntity,
};

pub async fn find_credit_note_scoped(
    db: &DatabaseConnection,
    id: i64,
) -> Option<credit_note::Model> {
    let query =
        scope_allowed::<super::routes::FinanceCreditNotesView, _>(CreditNoteEntity::find_by_id(id));
    lariv_core::web::opt_or_log(query.one(db).await, "find credit note scoped")
}

pub fn order_credit_notes(query: Select<CreditNoteEntity>) -> Select<CreditNoteEntity> {
    query.order_by_desc(credit_note::Column::Id)
}
