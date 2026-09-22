//! Startup seed helpers for HR plugin roles.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter,
};

use crate::plugins::users::entities::role::{self, Entity as RoleEntity};

use super::roles;

async fn next_role_id<C: ConnectionTrait>(conn: &C) -> Result<i64, sea_orm::DbErr> {
    let max_id = RoleEntity::find()
        .all(conn)
        .await?
        .into_iter()
        .map(|role| role.id)
        .max()
        .unwrap_or(0);
    Ok(max_id + 1)
}

async fn ensure_role<C: ConnectionTrait>(conn: &C, name: &str) -> Result<(), sea_orm::DbErr> {
    if RoleEntity::find()
        .filter(role::Column::Name.eq(name))
        .one(conn)
        .await?
        .is_some()
    {
        return Ok(());
    }

    let now = Utc::now();
    let model = role::ActiveModel {
        id: Set(next_role_id(conn).await?),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(name.into()),
    };

    match model.insert(conn).await {
        Ok(_) => Ok(()),
        Err(err) => {
            if RoleEntity::find()
                .filter(role::Column::Name.eq(name))
                .one(conn)
                .await?
                .is_some()
            {
                Ok(())
            } else {
                Err(err)
            }
        }
    }
}

pub async fn ensure_hr_roles<C: ConnectionTrait>(conn: &C) -> Result<(), sea_orm::DbErr> {
    for &name in roles::ALL {
        ensure_role(conn, name).await?;
    }
    Ok(())
}

pub async fn seed(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    ensure_hr_roles(db).await
}
