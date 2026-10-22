use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::entities::service_preferences::{self, Entity as ServicePreferencesEntity};

pub fn optional_i64(v: Option<i64>) -> i64 {
    v.unwrap_or(0).max(0)
}

pub async fn load_service_preferences(db: &DatabaseConnection) -> service_preferences::Model {
    if let Ok(Some(p)) = ServicePreferencesEntity::find_by_id(1i64).one(db).await {
        return p;
    }
    let now = Utc::now();
    let am = service_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };
    am.insert(db).await.unwrap_or(service_preferences::Model {
        id: 1,
        created_at: Some(now),
        updated_at: Some(now),
        cost_of_sales_account_id: None,
        cost_credit_account_id: None,
    })
}

pub async fn set_service_tax_ids(
    db: &DatabaseConnection,
    service_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    use crate::entities::service_tax::{self, Entity as ServiceTaxEntity};
    ServiceTaxEntity::delete_many()
        .filter(service_tax::Column::ServiceId.eq(service_id))
        .exec(db)
        .await?;
    for tax_id in tax_ids {
        let am = service_tax::ActiveModel {
            service_id: Set(service_id),
            tax_id: Set(*tax_id),
        };
        am.insert(db).await?;
    }
    Ok(())
}

pub async fn load_service_tax_ids(db: &DatabaseConnection, service_id: i64) -> Vec<i64> {
    use crate::entities::service_tax::{self, Entity as ServiceTaxEntity};
    ServiceTaxEntity::find()
        .filter(service_tax::Column::ServiceId.eq(service_id))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.tax_id)
        .collect()
}

pub async fn load_default_service_tax_ids(db: &DatabaseConnection) -> Vec<i64> {
    use crate::entities::service_preferences_tax::{self, Entity as ServicePreferencesTaxEntity};
    ServicePreferencesTaxEntity::find()
        .filter(service_preferences_tax::Column::ServicePreferencesId.eq(1i64))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.tax_id)
        .collect()
}
