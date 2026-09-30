use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Select,
    sea_query::Expr,
};

use crate::plugins::users::state::AuthContext;

use super::document_type::DocumentType;
use super::entities::{
    aadhar_card::{self, Entity as AadharCardEntity},
    document::{self, Entity as DocumentEntity},
};

pub fn scope_documents(
    query: Select<DocumentEntity>,
    auth: &AuthContext,
) -> Select<DocumentEntity> {
    if auth.user.is_superuser {
        return query;
    }
    query.filter(Expr::cust("1 = 0"))
}

pub fn apply_document_sort(
    query: Select<DocumentEntity>,
    sort: Option<&str>,
) -> Select<DocumentEntity> {
    match sort.unwrap_or("").trim() {
        s if s.eq_ignore_ascii_case("Type DESC") => {
            query.order_by_desc(document::Column::DocumentType)
        }
        s if s.eq_ignore_ascii_case("Type ASC") || s.eq_ignore_ascii_case("Type") => {
            query.order_by_asc(document::Column::DocumentType)
        }
        _ => query.order_by_desc(document::Column::Id),
    }
}

/// Restrict the document query to rows whose type table matches `name`.
///
/// Today that is `aadhar_cards`. An empty match yields no documents.
pub async fn apply_name_filter(
    db: &DatabaseConnection,
    query: Select<DocumentEntity>,
    name: Option<&str>,
) -> Result<Select<DocumentEntity>, sea_orm::DbErr> {
    let Some(name) = name.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(query);
    };
    let ids: Vec<i32> = AadharCardEntity::find()
        .filter(aadhar_card::Column::Name.contains(name))
        .all(db)
        .await?
        .into_iter()
        .map(|card| card.id)
        .collect();
    if ids.is_empty() {
        return Ok(query.filter(Expr::cust("1 = 0")));
    }
    Ok(query.filter(
        Condition::all()
            .add(document::Column::DocumentType.eq(DocumentType::AadharCard))
            .add(document::Column::DocumentTypeId.is_in(ids)),
    ))
}

pub async fn find_document_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<document::Model> {
    let query = DocumentEntity::find_by_id(id);
    crate::web::opt_or_log(
        scope_documents(query, auth).one(db).await,
        "find document scoped",
    )
}
