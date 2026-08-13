use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::plugins::finance_products::entities::product_preferences::{
    self, Entity as ProductPreferencesEntity,
};

pub fn optional_i64(v: Option<i64>) -> i64 {
    v.unwrap_or(0).max(0)
}

pub async fn load_product_preferences(db: &DatabaseConnection) -> product_preferences::Model {
    if let Ok(Some(p)) = ProductPreferencesEntity::find_by_id(1i64).one(db).await {
        return p;
    }
    let now = Utc::now();
    let am = product_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };
    am.insert(db).await.unwrap_or(product_preferences::Model {
        id: 1,
        created_at: Some(now),
        updated_at: Some(now),
        inventory_account_id: None,
        cost_of_sales_account_id: None,
    })
}

pub async fn set_product_tax_ids(
    db: &DatabaseConnection,
    product_id: i64,
    tax_ids: &[i64],
) -> Result<(), sea_orm::DbErr> {
    use crate::plugins::finance_products::entities::product_tax::{
        self, Entity as ProductTaxEntity,
    };
    ProductTaxEntity::delete_many()
        .filter(product_tax::Column::ProductId.eq(product_id))
        .exec(db)
        .await?;
    for tax_id in tax_ids {
        let am = product_tax::ActiveModel {
            product_id: Set(product_id),
            tax_id: Set(*tax_id),
        };
        am.insert(db).await?;
    }
    Ok(())
}

pub async fn load_product_tax_ids(db: &DatabaseConnection, product_id: i64) -> Vec<i64> {
    use crate::plugins::finance_products::entities::product_tax::{
        self, Entity as ProductTaxEntity,
    };
    ProductTaxEntity::find()
        .filter(product_tax::Column::ProductId.eq(product_id))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.tax_id)
        .collect()
}

pub async fn load_default_product_tax_ids(db: &DatabaseConnection) -> Vec<i64> {
    use crate::plugins::finance_products::entities::product_preferences_tax::{
        self, Entity as ProductPreferencesTaxEntity,
    };
    ProductPreferencesTaxEntity::find()
        .filter(product_preferences_tax::Column::ProductPreferencesId.eq(1i64))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.tax_id)
        .collect()
}
