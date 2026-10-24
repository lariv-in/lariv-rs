use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use crate::entities::tax_preferences::{self, Entity as TaxPreferencesEntity};

pub async fn load_tax_preferences(db: &DatabaseConnection) -> tax_preferences::Model {
    if let Ok(Some(p)) = TaxPreferencesEntity::find_by_id(1i64).one(db).await {
        return p;
    }
    let now = Utc::now();
    let am = tax_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
    };
    am.insert(db).await.unwrap_or(tax_preferences::Model {
        id: 1,
        created_at: Some(now),
        updated_at: Some(now),
    })
}
