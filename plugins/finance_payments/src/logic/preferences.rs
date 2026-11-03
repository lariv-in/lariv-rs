use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, EntityTrait};

use crate::entities::payment_preferences::{self, Entity as PaymentPreferencesEntity};

pub async fn load_payment_preferences<C: ConnectionTrait>(db: &C) -> payment_preferences::Model {
    if let Ok(Some(p)) = PaymentPreferencesEntity::find_by_id(1i64).one(db).await {
        return p;
    }
    let now = Utc::now();
    let am = payment_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
    };
    am.insert(db).await.unwrap_or(payment_preferences::Model {
        id: 1,
        created_at: Some(now),
        updated_at: Some(now),
    })
}

pub async fn validate_payment_preferences_for_create<C: ConnectionTrait>(
    db: &C,
) -> Result<(), String> {
    match crate::payment_journal_template::load_template(db).await? {
        Some(_) => Ok(()),
        None => Err("payment preferences need a journal entry template".into()),
    }
}
