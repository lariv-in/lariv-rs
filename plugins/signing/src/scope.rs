//! Owner-only access to `user_signatures`.
//!
//! Superuser does not widen this scope. A caller only ever loads the row whose
//! `user_id` is their own.

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Select};

use lariv_plugin_users::state::AuthContext;

use super::entities::user_signature::{self, Entity as UserSignatureEntity, Model};

pub fn scope_own(
    query: Select<UserSignatureEntity>,
    auth: &AuthContext,
) -> Select<UserSignatureEntity> {
    query.filter(user_signature::Column::UserId.eq(auth.user.id))
}

pub async fn find_own_signature(db: &DatabaseConnection, auth: &AuthContext) -> Option<Model> {
    lariv_core::web::opt_or_log(
        scope_own(UserSignatureEntity::find(), auth).one(db).await,
        "find own user signature",
    )
}
