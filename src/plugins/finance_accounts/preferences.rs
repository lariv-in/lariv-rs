//! Singleton accounting preferences (`id = 1`).

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use crate::plugins::finance_accounts::entities::accounting_preferences::{self, Entity as AccountingPreferencesEntity};

pub async fn load_accounting_preferences(
    db: &DatabaseConnection,
) -> accounting_preferences::Model {
    if let Ok(Some(p)) = AccountingPreferencesEntity::find_by_id(1i64).one(db).await {
        return p;
    }
    let now = Utc::now();
    let am = accounting_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        default_currency_id: Set(None),
    };
    am.insert(db).await.unwrap_or(accounting_preferences::Model {
        id: 1,
        created_at: Some(now),
        updated_at: Some(now),
        default_currency_id: None,
    })
}

pub async fn save_default_currency_id(
    db: &DatabaseConnection,
    default_currency_id: Option<i64>,
) -> Result<(), sea_orm::DbErr> {
    let prefs = load_accounting_preferences(db).await;
    let now = Utc::now();
    let mut am: accounting_preferences::ActiveModel = prefs.into();
    am.updated_at = Set(Some(now));
    am.default_currency_id = Set(default_currency_id);
    am.update(db).await?;
    Ok(())
}
