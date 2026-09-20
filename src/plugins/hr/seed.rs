//! Startup seed helpers for HR plugin roles.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::plugins::users::entities::role::{self, Entity as RoleEntity};

use super::roles;

async fn ensure_role(db: &DatabaseConnection, name: &str) -> Result<(), sea_orm::DbErr> {
    if RoleEntity::find()
        .filter(role::Column::Name.eq(name))
        .one(db)
        .await?
        .is_some()
    {
        return Ok(());
    }

    let now = Utc::now();
    let model = role::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(name.into()),
    };
    model.insert(db).await?;
    Ok(())
}

pub async fn seed(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    for &name in roles::ALL {
        ensure_role(db, name).await?;
    }
    Ok(())
}
