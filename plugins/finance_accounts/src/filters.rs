//! Catalogs for account, currency, and journal list filters.

use sea_orm::sea_query::Condition;
use sea_orm::{ColumnTrait, DatabaseBackend};

use lariv_core::components::FilterClauseView;
use lariv_core::list_filters::{
    FilterClause, FilterField, FilterRowError, FilterWidget, apply_filters, bool_column, custom,
    number_column, parse_list_filters, text_column,
};

use crate::balance_type::BalanceType;
use crate::entities::{account, currency, journal};

pub struct PreparedFilters {
    pub fields: Vec<FilterField>,
    pub clauses: Vec<FilterClause>,
    pub filters: Vec<FilterClauseView>,
    pub errors: Vec<FilterRowError>,
    pub condition: Condition,
}

pub fn prepare(query: &str, fields: Vec<FilterField>, backend: DatabaseBackend) -> PreparedFilters {
    let clauses = parse_list_filters(query);
    let applied = apply_filters(backend, &fields, &clauses);
    let filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    PreparedFilters {
        fields,
        clauses,
        filters,
        errors: applied.errors,
        condition: applied.condition,
    }
}

fn balance_type_field() -> FilterField {
    let choices = crate::forms::balance_type_choices();
    let listed = choices.clone();
    custom(
        "balance_type",
        "Balance type",
        FilterWidget::Choice,
        choices,
        false,
        move |clause, _| {
            let raw = clause.value();
            if !listed.iter().any(|(key, _)| key == raw) {
                return Err(format!("unknown value: {raw}"));
            }
            let kind = BalanceType::parse(raw).ok_or_else(|| format!("unknown value: {raw}"))?;
            Ok(Condition::all().add(account::Column::BalanceType.eq(kind)))
        },
    )
}

pub fn account_list_fields() -> Vec<FilterField> {
    vec![
        text_column("name", "Name", account::Column::Name),
        number_column("code", "Code", account::Column::Code),
        bool_column("is_group", "Group account", account::Column::IsGroup),
        balance_type_field(),
    ]
}

pub fn account_select_fields() -> Vec<FilterField> {
    vec![
        text_column("name", "Name", account::Column::Name),
        number_column("code", "Code", account::Column::Code),
        balance_type_field(),
    ]
}

/// True when a name, code, balance type, or group clause should search the whole tree.
pub fn account_search_active(clauses: &[FilterClause]) -> bool {
    clauses.iter().any(|clause| match clause.field.as_str() {
        "name" => !clause.value().is_empty(),
        "code" => clause.value().parse::<i32>().is_ok(),
        "balance_type" => BalanceType::parse(clause.value()).is_some(),
        "is_group" => matches!(
            clause.value().trim().to_ascii_lowercase().as_str(),
            "1" | "0" | "true" | "false" | "yes" | "no"
        ),
        _ => false,
    })
}

pub fn currency_fields(include_minor_unit: bool) -> Vec<FilterField> {
    let mut fields = vec![
        number_column("code", "Numeric code", currency::Column::Code),
        text_column("name", "Name", currency::Column::Name),
        text_column("symbol", "Symbol", currency::Column::Symbol),
    ];
    if include_minor_unit {
        fields.push(number_column(
            "minor_unit",
            "Minor unit",
            currency::Column::MinorUnit,
        ));
    }
    fields
}

pub fn journal_fields() -> Vec<FilterField> {
    vec![
        text_column("name", "Name", journal::Column::Name),
        bool_column("is_active", "Active", journal::Column::IsActive),
        number_column("currency_id", "Currency ID", journal::Column::CurrencyId),
    ]
}
