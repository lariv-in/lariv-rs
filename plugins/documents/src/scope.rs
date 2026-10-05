use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Select,
    sea_query::Expr,
};

use super::document_type::DocumentType;
use super::entities::{
    aadhar_card::{self, Entity as AadharCardEntity},
    document::{self, Entity as DocumentEntity},
    pan_card::{self, Entity as PanCardEntity},
    passport::{self, Entity as PassportEntity},
};

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

/// Restrict the document query to rows whose type table name contains `name`.
pub async fn apply_name_filter(
    db: &DatabaseConnection,
    query: Select<DocumentEntity>,
    name: Option<&str>,
) -> Result<Select<DocumentEntity>, sea_orm::DbErr> {
    let Some(name) = name.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(query);
    };
    let aadhar_ids = ids_of(
        AadharCardEntity::find()
            .filter(aadhar_card::Column::Name.contains(name))
            .all(db)
            .await?,
    );
    let pan_ids = ids_of(
        PanCardEntity::find()
            .filter(pan_card::Column::Name.contains(name))
            .all(db)
            .await?,
    );
    let passport_ids = ids_of(
        PassportEntity::find()
            .filter(passport::Column::Name.contains(name))
            .all(db)
            .await?,
    );
    let mut matched = Condition::any();
    let mut any = false;
    if !aadhar_ids.is_empty() {
        any = true;
        matched = matched.add(
            Condition::all()
                .add(document::Column::DocumentType.eq(DocumentType::AadharCard))
                .add(document::Column::DocumentTypeId.is_in(aadhar_ids)),
        );
    }
    if !pan_ids.is_empty() {
        any = true;
        matched = matched.add(
            Condition::all()
                .add(document::Column::DocumentType.eq(DocumentType::Pan))
                .add(document::Column::DocumentTypeId.is_in(pan_ids)),
        );
    }
    if !passport_ids.is_empty() {
        any = true;
        matched = matched.add(
            Condition::all()
                .add(document::Column::DocumentType.eq(DocumentType::Passport))
                .add(document::Column::DocumentTypeId.is_in(passport_ids)),
        );
    }
    if !any {
        return Ok(query.filter(Expr::cust("1 = 0")));
    }
    Ok(query.filter(matched))
}

fn ids_of(rows: impl IntoIterator<Item = impl HasId>) -> Vec<i32> {
    rows.into_iter().map(|row| row.row_id()).collect()
}

trait HasId {
    fn row_id(&self) -> i32;
}

impl HasId for aadhar_card::Model {
    fn row_id(&self) -> i32 {
        self.id
    }
}

impl HasId for pan_card::Model {
    fn row_id(&self) -> i32 {
        self.id
    }
}

impl HasId for passport::Model {
    fn row_id(&self) -> i32 {
        self.id
    }
}

pub async fn find_document_scoped(db: &DatabaseConnection, id: i64) -> Option<document::Model> {
    let query = scope_allowed::<super::routes::DocumentsView, _>(DocumentEntity::find_by_id(id));
    lariv_core::web::opt_or_log(query.one(db).await, "find document scoped")
}
