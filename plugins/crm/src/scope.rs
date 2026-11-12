use lariv_core::components::{sort_desc, sort_key};
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait, JoinType,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait, Select,
    sea_query::{Expr, ExprTrait, Order, Query as SeaQuery, SelectStatement},
};

use lariv_plugin_contacts::entities::{
    company::{self, Entity as CompanyEntity},
    contact::{self, Entity as ContactEntity},
};
use lariv_plugin_contacts::routes::{CompanyDetailRouteTag, ContactDetailRouteTag};
use lariv_plugin_customer::entities::customer::{self, Entity as CustomerEntity};

use super::entities::{
    converted_lead::{self, Entity as ConvertedLeadEntity},
    failed_lead::{self, Entity as FailedLeadEntity},
    lead::{self, Entity as LeadEntity},
    lead_tag::{self, Entity as LeadTagEntity},
    lead_tag_link,
    lead_update::{self, Entity as LeadUpdateEntity},
};

fn converted_lead_ids_subquery() -> SelectStatement {
    SeaQuery::select()
        .column(converted_lead::Column::LeadId)
        .from(ConvertedLeadEntity)
        .to_owned()
}

fn failed_lead_ids_subquery() -> SelectStatement {
    SeaQuery::select()
        .column(failed_lead::Column::LeadId)
        .from(FailedLeadEntity)
        .to_owned()
}

/// Leads with no converted/failed row (same shape as overtime/leaves pending filters).
pub fn lead_active_condition() -> Condition {
    Condition::all()
        .add(lead::Column::Id.not_in_subquery(converted_lead_ids_subquery()))
        .add(lead::Column::Id.not_in_subquery(failed_lead_ids_subquery()))
}

pub async fn find_active_lead(db: &DatabaseConnection, id: i64) -> Option<lead::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<super::routes::CrmView, _>(LeadEntity::find_by_id(id))
            .filter(lead_active_condition())
            .one(db)
            .await,
        "find by id",
    )
}

pub async fn find_lead_scoped(db: &DatabaseConnection, id: i64) -> Option<lead::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<super::routes::CrmView, _>(LeadEntity::find_by_id(id))
            .one(db)
            .await,
        "find by id",
    )
}

pub async fn find_lead_update_scoped(
    db: &DatabaseConnection,
    id: i64,
) -> Option<lead_update::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<super::routes::CrmView, _>(LeadUpdateEntity::find_by_id(id))
            .one(db)
            .await,
        "find by id",
    )
}

pub async fn find_converted_lead_scoped(
    db: &DatabaseConnection,
    id: i64,
) -> Option<converted_lead::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<super::routes::CrmView, _>(ConvertedLeadEntity::find_by_id(id))
            .one(db)
            .await,
        "find by id",
    )
}

pub async fn find_failed_lead_scoped(
    db: &DatabaseConnection,
    id: i64,
) -> Option<failed_lead::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<super::routes::CrmView, _>(FailedLeadEntity::find_by_id(id))
            .one(db)
            .await,
        "find by id",
    )
}

pub async fn find_lead_tag_scoped(db: &DatabaseConnection, id: i64) -> Option<lead_tag::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<super::routes::CrmView, _>(LeadTagEntity::find_by_id(id))
            .one(db)
            .await,
        "find by id",
    )
}

/// Leads whose id column is in the set of leads that have any of `tag_ids`.
pub fn any_lead_tag_condition<C>(lead_id_col: C, tag_ids: &[i64]) -> Condition
where
    C: sea_orm::sea_query::IntoColumnRef,
{
    let tag_ids: Vec<i64> = tag_ids.iter().copied().filter(|id| *id > 0).collect();
    if tag_ids.is_empty() {
        return Condition::all();
    }
    Condition::all().add(Expr::col(lead_id_col).in_subquery(lead_ids_with_tags_subquery(&tag_ids)))
}

/// Leads that have any of the selected tags.
pub fn apply_lead_tag_id_filter<E, C>(
    query: Select<E>,
    lead_id_col: C,
    tag_ids: &[i64],
) -> Select<E>
where
    E: EntityTrait,
    C: ColumnTrait + sea_orm::sea_query::IntoColumnRef,
{
    let tag_ids: Vec<i64> = tag_ids.iter().copied().filter(|id| *id > 0).collect();
    if tag_ids.is_empty() {
        return query;
    }
    query.filter(any_lead_tag_condition(lead_id_col, &tag_ids))
}

fn lead_ids_with_tags_subquery(tag_ids: &[i64]) -> SelectStatement {
    let mut sub = SeaQuery::select();
    sub.column(lead_tag_link::Column::LeadId)
        .from(lead_tag_link::Entity)
        .and_where(lead_tag_link::Column::LeadTagId.is_in(tag_ids.to_vec()));
    sub
}

pub fn join_active_lead_relations(
    mut query: Select<LeadEntity>,
    sort: Option<&str>,
    needs_party: bool,
) -> Select<LeadEntity> {
    let key = sort_key(sort.unwrap_or(""));
    let need_party = needs_party
        || key.eq_ignore_ascii_case("Name")
        || key.eq_ignore_ascii_case("Email")
        || key.eq_ignore_ascii_case("Company");
    if need_party {
        query = join_lead_customer(query);
    }
    query
}

fn join_customer_party<E: EntityTrait>(
    mut query: Select<E>,
    customer_rel: sea_orm::entity::RelationDef,
) -> Select<E> {
    query = query.join(JoinType::LeftJoin, customer_rel);
    query = query.join(JoinType::LeftJoin, customer::Relation::Individual.def());
    query.join(JoinType::LeftJoin, customer::Relation::Company.def())
}

fn join_lead_customer<E: EntityTrait>(query: Select<E>) -> Select<E> {
    join_customer_party(query, lead::Relation::Customer.def())
}

pub fn join_converted_lead_relations(
    mut query: Select<ConvertedLeadEntity>,
    sort: Option<&str>,
    needs_contact: bool,
) -> Select<ConvertedLeadEntity> {
    let key = sort_key(sort.unwrap_or(""));
    let need_party = needs_contact
        || key.eq_ignore_ascii_case("Name")
        || key.eq_ignore_ascii_case("Email")
        || key.eq_ignore_ascii_case("Company");
    if need_party {
        query = join_customer_party(query, converted_lead::Relation::Customer.def());
    }
    if key.eq_ignore_ascii_case("Source") {
        query = query.join(JoinType::LeftJoin, converted_lead::Relation::Lead.def());
    }
    query
}

pub fn join_failed_lead_relations(
    mut query: Select<FailedLeadEntity>,
    sort: Option<&str>,
    needs_contact: bool,
) -> Select<FailedLeadEntity> {
    let key = sort_key(sort.unwrap_or(""));
    let need_party = needs_contact
        || key.eq_ignore_ascii_case("Name")
        || key.eq_ignore_ascii_case("Email")
        || key.eq_ignore_ascii_case("Company");
    if need_party {
        query = join_customer_party(query, failed_lead::Relation::Customer.def());
    }
    if key.eq_ignore_ascii_case("Source") {
        query = query.join(JoinType::LeftJoin, failed_lead::Relation::Lead.def());
    }
    query
}

pub fn apply_lead_sort(query: Select<LeadEntity>, sort: Option<&str>) -> Select<LeadEntity> {
    let sort = sort.unwrap_or("").trim();
    let desc = sort_desc(sort);
    match sort_key(sort) {
        s if s.eq_ignore_ascii_case("Name") => query.order_by(party_name_expr(), sort_order(desc)),
        s if s.eq_ignore_ascii_case("Company") => {
            if desc {
                query.order_by_desc(company::Column::Name)
            } else {
                query.order_by_asc(company::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("Email") => {
            if desc {
                query.order_by_desc(contact::Column::Email)
            } else {
                query.order_by_asc(contact::Column::Email)
            }
        }
        s if s.eq_ignore_ascii_case("Source") => {
            if desc {
                query.order_by_desc(lead::Column::Source)
            } else {
                query.order_by_asc(lead::Column::Source)
            }
        }
        _ => query.order_by_desc(lead::Column::Id),
    }
}

pub fn apply_converted_lead_sort(
    query: Select<ConvertedLeadEntity>,
    sort: Option<&str>,
) -> Select<ConvertedLeadEntity> {
    let sort = sort.unwrap_or("").trim();
    let key = sort_key(sort);
    let desc = sort_desc(sort);
    match key {
        s if s.eq_ignore_ascii_case("Name") => query.order_by(party_name_expr(), sort_order(desc)),
        s if s.eq_ignore_ascii_case("Company") => {
            if desc {
                query.order_by_desc(company::Column::Name)
            } else {
                query.order_by_asc(company::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("Email") => {
            if desc {
                query.order_by_desc(contact::Column::Email)
            } else {
                query.order_by_asc(contact::Column::Email)
            }
        }
        s if s.eq_ignore_ascii_case("Source") => {
            if desc {
                query.order_by_desc(lead::Column::Source)
            } else {
                query.order_by_asc(lead::Column::Source)
            }
        }
        _ => query.order_by_desc(converted_lead::Column::Id),
    }
}

pub fn apply_failed_lead_sort(
    query: Select<FailedLeadEntity>,
    sort: Option<&str>,
) -> Select<FailedLeadEntity> {
    let sort = sort.unwrap_or("").trim();
    let key = sort_key(sort);
    let desc = sort_desc(sort);
    match key {
        s if s.eq_ignore_ascii_case("Name") => query.order_by(party_name_expr(), sort_order(desc)),
        s if s.eq_ignore_ascii_case("Company") => {
            if desc {
                query.order_by_desc(company::Column::Name)
            } else {
                query.order_by_asc(company::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("Email") => {
            if desc {
                query.order_by_desc(contact::Column::Email)
            } else {
                query.order_by_asc(contact::Column::Email)
            }
        }
        s if s.eq_ignore_ascii_case("Source") => {
            if desc {
                query.order_by_desc(lead::Column::Source)
            } else {
                query.order_by_asc(lead::Column::Source)
            }
        }
        _ => query.order_by_desc(failed_lead::Column::Id),
    }
}

pub use lariv_plugin_contacts::scope::company_display_label;

/// Resolved customer party for lead list and detail views.
#[derive(Clone, Debug, Default)]
pub struct LeadPartyView {
    pub display_name: String,
    pub company: String,
    pub email: String,
    pub contact_id: i64,
    pub company_id: i64,
    pub customer_id: i64,
    pub href: String,
}

pub async fn lead_party_view<C: ConnectionTrait>(db: &C, customer_id: i64) -> LeadPartyView {
    if customer_id <= 0 {
        return LeadPartyView::default();
    }
    let Some(customer) = lariv_core::web::opt_or_log(
        CustomerEntity::find_by_id(customer_id).one(db).await,
        "find by id",
    ) else {
        return LeadPartyView {
            display_name: format!("Customer #{customer_id}"),
            customer_id,
            ..Default::default()
        };
    };
    if customer.is_individual {
        let Some(contact_id) = customer.individual_id.filter(|id| *id > 0) else {
            return LeadPartyView {
                display_name: format!("Customer #{customer_id}"),
                customer_id,
                ..Default::default()
            };
        };
        let Some(contact) = lariv_core::web::opt_or_log(
            ContactEntity::find_by_id(contact_id).one(db).await,
            "find by id",
        ) else {
            return LeadPartyView {
                display_name: format!("Customer #{customer_id}"),
                customer_id,
                contact_id,
                href: ContactDetailRouteTag::new(contact_id).url(),
                ..Default::default()
            };
        };
        let (company, company_id) = match contact.company_id {
            Some(id) => (
                lariv_core::web::opt_or_log(
                    CompanyEntity::find_by_id(id).one(db).await,
                    "find by id",
                )
                .map(|row| row.name)
                .unwrap_or_default(),
                id,
            ),
            None => (String::new(), 0),
        };
        LeadPartyView {
            display_name: contact.display_name(),
            company,
            email: contact.email.unwrap_or_default(),
            contact_id: contact.id,
            company_id,
            customer_id,
            href: ContactDetailRouteTag::new(contact.id).url(),
        }
    } else {
        let Some(company_id) = customer.company_id.filter(|id| *id > 0) else {
            return LeadPartyView {
                display_name: format!("Customer #{customer_id}"),
                customer_id,
                ..Default::default()
            };
        };
        let Some(company) = lariv_core::web::opt_or_log(
            CompanyEntity::find_by_id(company_id).one(db).await,
            "find by id",
        ) else {
            return LeadPartyView {
                display_name: format!("Customer #{customer_id}"),
                customer_id,
                company_id,
                href: CompanyDetailRouteTag::new(company_id).url(),
                ..Default::default()
            };
        };
        let contact = lariv_core::web::opt_or_log(
            ContactEntity::find()
                .filter(contact::Column::CompanyId.eq(company.id))
                .filter(contact::Column::IsPrimary.eq(true))
                .one(db)
                .await,
            "find primary contact",
        );
        LeadPartyView {
            display_name: company.name.clone(),
            email: company.email.clone().unwrap_or_default(),
            company: company.name,
            contact_id: contact.map(|row| row.id).unwrap_or(0),
            company_id: company.id,
            customer_id,
            href: CompanyDetailRouteTag::new(company.id).url(),
        }
    }
}

fn party_name_expr() -> Expr {
    Expr::case(
        Expr::col((CustomerEntity, customer::Column::IsIndividual)).eq(true),
        Expr::col((ContactEntity, contact::Column::Name)),
    )
    .finally(Expr::col((CompanyEntity, company::Column::Name)))
    .into()
}

fn sort_order(desc: bool) -> Order {
    if desc { Order::Desc } else { Order::Asc }
}

pub async fn lead_display_name(db: &DatabaseConnection, lead: &lead::Model) -> String {
    let view = lead_party_view(db, lead.customer_id).await;
    if view.display_name.is_empty() {
        format!("Lead #{}", lead.id)
    } else {
        view.display_name
    }
}
