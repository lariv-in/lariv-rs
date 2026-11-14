use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, EntityTrait};

use crate::entities::credit_note_preferences::{self, Entity as CreditNotePreferencesEntity};

pub async fn load_credit_note_preferences<C: ConnectionTrait>(
    db: &C,
) -> credit_note_preferences::Model {
    if let Ok(Some(prefs)) = CreditNotePreferencesEntity::find_by_id(1i64).one(db).await {
        return prefs;
    }
    let now = Utc::now();
    let am = credit_note_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        credit_note_number_format: Set(None),
    };
    match am.insert(db).await {
        Ok(prefs) => prefs,
        Err(_) => credit_note_preferences::Model {
            id: 1,
            created_at: Some(now),
            updated_at: Some(now),
            credit_note_number_format: None,
        },
    }
}
