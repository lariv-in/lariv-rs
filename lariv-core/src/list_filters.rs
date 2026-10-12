//! Dynamic list filters: indexed query clauses folded into one SeaORM condition.
//!
//! AND binds tighter than OR, so `A AND B OR C AND D` is `(A AND B) OR (C AND D)`.
//! The first clause's join is ignored. Text matches use [`text_match_condition`]
//! and do not change sort order. Clauses whose field is not in the catalog are ignored.

use std::collections::BTreeMap;

use sea_orm::DatabaseBackend;
use sea_orm::sea_query::{Condition, Expr, ExprTrait, IntoColumnRef, SimpleExpr, Value};

use crate::datetime::{DatetimeLocalInput, parse_date, parse_time};
use crate::db::trigram::text_match_condition;
use crate::duration::parse_duration;

/// How a clause combines with the previous clause.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinOp {
    And,
    Or,
}

impl JoinOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "and" => Some(Self::And),
            "or" => Some(Self::Or),
            _ => None,
        }
    }
}

/// Comparison for ordered values. Text, keys, choices, and booleans ignore this.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpOp {
    Lt,
    Gt,
    Eq,
}

impl CmpOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lt => "lt",
            Self::Gt => "gt",
            Self::Eq => "eq",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "lt" => Some(Self::Lt),
            "gt" => Some(Self::Gt),
            "eq" => Some(Self::Eq),
            _ => None,
        }
    }
}

/// One filter row, in query order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterClause {
    /// Index from `filter.{index}.*`, which is also the visual order.
    pub index: usize,
    /// Ignored for the first applied clause.
    pub join: JoinOp,
    pub field: String,
    /// Defaults to [`CmpOp::Eq`] when the query omits `op`.
    pub op: CmpOp,
    pub values: Vec<String>,
}

impl FilterClause {
    pub fn value(&self) -> &str {
        self.values.first().map(String::as_str).unwrap_or("")
    }
}

/// A value that was present but could not be parsed for its field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterRowError {
    pub index: usize,
    pub message: String,
}

/// Value control rendered for a catalog field.
#[derive(Clone, Debug)]
pub enum FilterWidget {
    Text,
    Number,
    Decimal,
    Date,
    Time,
    Datetime,
    Duration,
    ForeignKey {
        url: String,
        placeholder: String,
        search_key: String,
    },
    ManyToMany {
        url: String,
        placeholder: String,
    },
    Choice,
    Bool,
}

impl FilterWidget {
    pub fn has_operator(&self) -> bool {
        matches!(
            self,
            Self::Number
                | Self::Decimal
                | Self::Date
                | Self::Time
                | Self::Datetime
                | Self::Duration
        )
    }
}

type ApplyFn = Box<dyn Fn(&FilterClause, DatabaseBackend) -> Result<Condition, String> + Send>;

/// One field the filter builder can add.
pub struct FilterField {
    key: &'static str,
    label: &'static str,
    widget: FilterWidget,
    choices: Vec<(String, String)>,
    post_filter: bool,
    apply: ApplyFn,
}

impl FilterField {
    pub fn key(&self) -> &'static str {
        self.key
    }

    pub fn label(&self) -> &'static str {
        self.label
    }

    pub fn widget(&self) -> &FilterWidget {
        &self.widget
    }

    pub fn choices(&self) -> &[(String, String)] {
        &self.choices
    }

    pub fn has_operator(&self) -> bool {
        self.widget.has_operator()
    }

    /// When true, the clause is left out of the SQL condition for the handler to apply.
    pub fn is_post_filter(&self) -> bool {
        self.post_filter
    }

    fn apply_clause(
        &self,
        clause: &FilterClause,
        backend: DatabaseBackend,
    ) -> Result<Condition, String> {
        (self.apply)(clause, backend)
    }
}

/// SQL condition plus clauses the handler still has to apply itself.
#[derive(Debug)]
pub struct AppliedFilters {
    pub condition: Condition,
    pub errors: Vec<FilterRowError>,
    /// Post-filter clauses, in query order, whose values parsed.
    pub post: Vec<FilterClause>,
}

struct PartialRow {
    join: Option<JoinOp>,
    field: String,
    op: Option<CmpOp>,
    values: Vec<String>,
}

impl Default for PartialRow {
    fn default() -> Self {
        Self {
            join: None,
            field: String::new(),
            op: None,
            values: Vec::new(),
        }
    }
}

/// Parse `filter.{i}.join`, `filter.{i}.field`, `filter.{i}.op`, and repeated
/// `filter.{i}.value` from a raw query string.
///
/// Rows with an empty field or no values are dropped. Other query keys are ignored.
pub fn parse_list_filters(query: &str) -> Vec<FilterClause> {
    let mut rows: BTreeMap<usize, PartialRow> = BTreeMap::new();
    for (key, value) in form_urlencoded::parse(query.as_bytes()) {
        let Some(rest) = key.strip_prefix("filter.") else {
            continue;
        };
        let Some((index, part)) = rest.split_once('.') else {
            continue;
        };
        let Ok(index) = index.parse::<usize>() else {
            continue;
        };
        let row = rows.entry(index).or_default();
        match part {
            "join" => row.join = JoinOp::parse(&value),
            "field" => row.field = value.into_owned(),
            "op" => row.op = CmpOp::parse(&value),
            "value" => row.values.push(value.into_owned()),
            _ => {}
        }
    }
    rows.into_iter()
        .filter_map(|(index, row)| {
            let field = row.field.trim().to_string();
            let values: Vec<String> = row
                .values
                .into_iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect();
            if field.is_empty() || values.is_empty() {
                return None;
            }
            Some(FilterClause {
                index,
                join: row.join.unwrap_or(JoinOp::And),
                field,
                op: row.op.unwrap_or(CmpOp::Eq),
                values,
            })
        })
        .collect()
}

/// Fold catalog clauses into one condition.
///
/// Unknown fields are ignored. A clause whose value does not parse is recorded
/// in [`AppliedFilters::errors`] and left out of the condition. Post-filter
/// fields are recorded in [`AppliedFilters::post`] and left out of the SQL grouping.
pub fn apply_filters(
    backend: DatabaseBackend,
    fields: &[FilterField],
    clauses: &[FilterClause],
) -> AppliedFilters {
    let mut errors = Vec::new();
    let mut post = Vec::new();
    let mut parts: Vec<(JoinOp, Condition)> = Vec::new();
    for clause in clauses {
        let Some(field) = fields.iter().find(|field| field.key == clause.field) else {
            continue;
        };
        if clause.value().is_empty() {
            continue;
        }
        match field.apply_clause(clause, backend) {
            Ok(_) if field.is_post_filter() => post.push(clause.clone()),
            Ok(condition) => parts.push((clause.join, condition)),
            Err(message) => errors.push(FilterRowError {
                index: clause.index,
                message,
            }),
        }
    }
    AppliedFilters {
        condition: fold_conditions(parts),
        errors,
        post,
    }
}

/// AND-groups separated by OR. The first clause does not consult its join.
pub fn fold_conditions(parts: Vec<(JoinOp, Condition)>) -> Condition {
    if parts.is_empty() {
        return Condition::all();
    }
    let mut groups: Vec<Condition> = Vec::new();
    let mut current = Condition::all();
    for (i, (join, condition)) in parts.into_iter().enumerate() {
        if i == 0 || join == JoinOp::And {
            current = current.add(condition);
        } else {
            groups.push(std::mem::replace(
                &mut current,
                Condition::all().add(condition),
            ));
        }
    }
    groups.push(current);
    if groups.len() == 1 {
        return groups.pop().unwrap_or_else(Condition::all);
    }
    let mut any = Condition::any();
    for group in groups {
        any = any.add(group);
    }
    any
}

fn compare<C, V>(col: C, op: CmpOp, value: V) -> SimpleExpr
where
    C: IntoColumnRef,
    V: Into<Value>,
{
    let expr = Expr::col(col);
    match op {
        CmpOp::Lt => expr.lt(value),
        CmpOp::Gt => expr.gt(value),
        CmpOp::Eq => expr.eq(value),
    }
}

fn one(expr: SimpleExpr) -> Condition {
    Condition::all().add(expr)
}

fn known_choice(choices: &[(String, String)], raw: &str) -> Result<(), String> {
    if choices.is_empty() || choices.iter().any(|(key, _)| key == raw) {
        Ok(())
    } else {
        Err(format!("unknown value: {raw}"))
    }
}

/// Case-insensitive trigram match. The condition does not order rows.
pub fn text_column<C>(key: &'static str, label: &'static str, col: C) -> FilterField
where
    C: Copy + IntoColumnRef + Send + Sync + 'static,
{
    FilterField {
        key,
        label,
        widget: FilterWidget::Text,
        choices: Vec::new(),
        post_filter: false,
        apply: Box::new(move |clause, backend| {
            let needle = clause.value().trim();
            if needle.is_empty() {
                return Ok(Condition::all());
            }
            Ok(text_match_condition(backend, &[col], needle))
        }),
    }
}

/// Integer column with less than, greater than, and equals.
pub fn number_column<C>(key: &'static str, label: &'static str, col: C) -> FilterField
where
    C: Copy + IntoColumnRef + Send + Sync + 'static,
{
    ordered(key, label, FilterWidget::Number, move |clause| {
        let raw = clause.value();
        let parsed = raw
            .parse::<i64>()
            .map_err(|_| format!("invalid number: {raw}"))?;
        Ok(one(compare(col, clause.op, parsed)))
    })
}

/// Decimal column with less than, greater than, and equals.
pub fn decimal_column<C>(key: &'static str, label: &'static str, col: C) -> FilterField
where
    C: Copy + IntoColumnRef + Send + Sync + 'static,
{
    ordered(key, label, FilterWidget::Decimal, move |clause| {
        let raw = clause.value();
        let parsed = parse_decimal(raw)?;
        Ok(one(compare(col, clause.op, parsed)))
    })
}

/// Calendar date column with less than, greater than, and equals.
pub fn date_column<C>(key: &'static str, label: &'static str, col: C) -> FilterField
where
    C: Copy + IntoColumnRef + Send + Sync + 'static,
{
    ordered(key, label, FilterWidget::Date, move |clause| {
        let raw = clause.value();
        let parsed = parse_date(raw).ok_or_else(|| format!("invalid date: {raw}"))?;
        Ok(one(compare(col, clause.op, parsed)))
    })
}

/// Clock time column with less than, greater than, and equals.
pub fn time_column<C>(key: &'static str, label: &'static str, col: C) -> FilterField
where
    C: Copy + IntoColumnRef + Send + Sync + 'static,
{
    ordered(key, label, FilterWidget::Time, move |clause| {
        let raw = clause.value();
        let parsed = parse_time(raw).ok_or_else(|| format!("invalid time: {raw}"))?;
        Ok(one(compare(col, clause.op, parsed)))
    })
}

/// UTC timestamp column. The input is wall time in `tz`.
pub fn datetime_column<C>(
    key: &'static str,
    label: &'static str,
    col: C,
    tz: impl Into<String>,
) -> FilterField
where
    C: Copy + IntoColumnRef + Send + Sync + 'static,
{
    let tz = tz.into();
    ordered(key, label, FilterWidget::Datetime, move |clause| {
        let raw = clause.value();
        let parsed = DatetimeLocalInput::from_raw(raw)
            .to_stored(&tz)
            .ok_or_else(|| format!("invalid date: {raw}"))?;
        Ok(one(compare(col, clause.op, parsed)))
    })
}

/// Duration column stored as nanoseconds, with less than, greater than, and equals.
pub fn duration_column<C>(key: &'static str, label: &'static str, col: C) -> FilterField
where
    C: Copy + IntoColumnRef + Send + Sync + 'static,
{
    ordered(key, label, FilterWidget::Duration, move |clause| {
        let raw = clause.value();
        let parsed = parse_duration(raw)?;
        Ok(one(compare(col, clause.op, parsed)))
    })
}

/// Foreign-key id equality. `url` is the picker table route.
pub fn foreign_key_column<C>(
    key: &'static str,
    label: &'static str,
    col: C,
    url: impl Into<String>,
    placeholder: impl Into<String>,
) -> FilterField
where
    C: Copy + IntoColumnRef + Send + Sync + 'static,
{
    let url = url.into();
    let placeholder = placeholder.into();
    FilterField {
        key,
        label,
        widget: FilterWidget::ForeignKey {
            url,
            placeholder,
            search_key: "Name".into(),
        },
        choices: Vec::new(),
        post_filter: false,
        apply: Box::new(move |clause, _| {
            let raw = clause.value();
            let id = raw
                .parse::<i64>()
                .map_err(|_| format!("invalid id: {raw}"))?;
            if id <= 0 {
                return Err(format!("invalid id: {raw}"));
            }
            Ok(one(compare(col, CmpOp::Eq, id)))
        }),
    }
}

/// Set the typeahead query parameter on a foreign-key field. Default is `Name`.
pub fn with_search_key(mut field: FilterField, search_key: impl Into<String>) -> FilterField {
    if let FilterWidget::ForeignKey {
        search_key: slot, ..
    } = &mut field.widget
    {
        *slot = search_key.into();
    }
    field
}

/// String column equality against `choices` keys. An empty choice list accepts any value.
pub fn choice_column<C>(
    key: &'static str,
    label: &'static str,
    col: C,
    choices: Vec<(String, String)>,
) -> FilterField
where
    C: Copy + IntoColumnRef + Send + Sync + 'static,
{
    let listed = choices.clone();
    FilterField {
        key,
        label,
        widget: FilterWidget::Choice,
        choices,
        post_filter: false,
        apply: Box::new(move |clause, _| {
            let raw = clause.value().to_string();
            known_choice(&listed, &raw)?;
            Ok(one(compare(col, CmpOp::Eq, raw)))
        }),
    }
}

/// Integer column equality against `choices` keys.
pub fn choice_i64_column<C>(
    key: &'static str,
    label: &'static str,
    col: C,
    choices: Vec<(String, String)>,
) -> FilterField
where
    C: Copy + IntoColumnRef + Send + Sync + 'static,
{
    let listed = choices.clone();
    FilterField {
        key,
        label,
        widget: FilterWidget::Choice,
        choices,
        post_filter: false,
        apply: Box::new(move |clause, _| {
            let raw = clause.value();
            known_choice(&listed, raw)?;
            let id = raw
                .parse::<i64>()
                .map_err(|_| format!("invalid number: {raw}"))?;
            Ok(one(compare(col, CmpOp::Eq, id)))
        }),
    }
}

/// Boolean column equality. The control submits `1` or `0`.
pub fn bool_column<C>(key: &'static str, label: &'static str, col: C) -> FilterField
where
    C: Copy + IntoColumnRef + Send + Sync + 'static,
{
    FilterField {
        key,
        label,
        widget: FilterWidget::Bool,
        choices: Vec::new(),
        post_filter: false,
        apply: Box::new(move |clause, _| {
            let parsed = parse_bool(clause.value())?;
            Ok(one(compare(col, CmpOp::Eq, parsed)))
        }),
    }
}

/// Many-to-many control. `apply` builds the relation condition from the selected ids.
pub fn many_to_many<F>(
    key: &'static str,
    label: &'static str,
    url: impl Into<String>,
    placeholder: impl Into<String>,
    apply: F,
) -> FilterField
where
    F: Fn(&FilterClause, DatabaseBackend) -> Result<Condition, String> + Send + 'static,
{
    FilterField {
        key,
        label,
        widget: FilterWidget::ManyToMany {
            url: url.into(),
            placeholder: placeholder.into(),
        },
        choices: Vec::new(),
        post_filter: false,
        apply: Box::new(apply),
    }
}

/// A field whose condition is not a single column.
///
/// Set `post_filter` when the handler must apply the clause after the query
/// (the SQL condition then ignores it). `apply` still runs so a bad value can
/// be rejected before that.
pub fn custom<F>(
    key: &'static str,
    label: &'static str,
    widget: FilterWidget,
    choices: Vec<(String, String)>,
    post_filter: bool,
    apply: F,
) -> FilterField
where
    F: Fn(&FilterClause, DatabaseBackend) -> Result<Condition, String> + Send + 'static,
{
    FilterField {
        key,
        label,
        widget,
        choices,
        post_filter,
        apply: Box::new(apply),
    }
}

/// Parse selected many-to-many ids. An empty selection is an error only when
/// the clause has values that are not integers.
pub fn parse_id_list(clause: &FilterClause) -> Result<Vec<i64>, String> {
    let mut ids = Vec::with_capacity(clause.values.len());
    for raw in &clause.values {
        let id = raw
            .parse::<i64>()
            .map_err(|_| format!("invalid id: {raw}"))?;
        if id <= 0 {
            return Err(format!("invalid id: {raw}"));
        }
        ids.push(id);
    }
    Ok(ids)
}

/// Clauses for `key`, in query order.
pub fn clauses_for<'a>(clauses: &'a [FilterClause], key: &str) -> Vec<&'a FilterClause> {
    clauses
        .iter()
        .filter(|clause| clause.field == key)
        .collect()
}

fn ordered<F>(key: &'static str, label: &'static str, widget: FilterWidget, apply: F) -> FilterField
where
    F: Fn(&FilterClause) -> Result<Condition, String> + Send + 'static,
{
    FilterField {
        key,
        label,
        widget,
        choices: Vec::new(),
        post_filter: false,
        apply: Box::new(move |clause, _| apply(clause)),
    }
}

fn parse_decimal(raw: &str) -> Result<sea_orm::prelude::Decimal, String> {
    use std::str::FromStr;
    sea_orm::prelude::Decimal::from_str(raw.trim()).map_err(|_| format!("invalid number: {raw}"))
}

fn parse_bool(raw: &str) -> Result<bool, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" => Ok(true),
        "0" | "false" | "no" => Ok(false),
        _ => Err(format!("invalid yes/no: {raw}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Iden;
    use sea_orm::sea_query::{Alias, PostgresQueryBuilder, Query, SqliteQueryBuilder};

    #[derive(Clone, Copy)]
    struct NameCol;

    impl Iden for NameCol {
        fn unquoted(&self) -> &str {
            "name"
        }
    }

    fn sql(condition: Condition) -> String {
        Query::select()
            .column(Alias::new("id"))
            .from(Alias::new("t"))
            .cond_where(condition)
            .to_string(PostgresQueryBuilder)
    }

    fn clause(index: usize, join: JoinOp, field: &str, op: CmpOp, value: &str) -> FilterClause {
        FilterClause {
            index,
            join,
            field: field.into(),
            op,
            values: vec![value.into()],
        }
    }

    fn flag(name: &'static str) -> FilterField {
        custom(
            name,
            name,
            FilterWidget::Text,
            Vec::new(),
            false,
            move |clause, _| {
                Ok(one(
                    Expr::col(Alias::new(name)).eq(clause.value().to_string())
                ))
            },
        )
    }

    #[test]
    fn parse_keeps_order_ops_and_repeated_values() {
        let parsed = parse_list_filters(
            "page=2&filter.1.join=or&filter.1.field=tags&filter.1.value=2&filter.1.value=4&filter.0.field=title&filter.0.value=Hello&sort=Name+ASC",
        );
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].field, "title");
        assert_eq!(parsed[0].values, vec!["Hello".to_string()]);
        assert_eq!(parsed[0].op, CmpOp::Eq);
        assert_eq!(parsed[1].join, JoinOp::Or);
        assert_eq!(parsed[1].field, "tags");
        assert_eq!(parsed[1].values, vec!["2".to_string(), "4".to_string()]);
    }

    #[test]
    fn parse_drops_empty_rows() {
        let parsed = parse_list_filters(
            "filter.0.field=title&filter.0.value=&filter.2.field=&filter.2.value=x",
        );
        assert!(parsed.is_empty());
    }

    #[test]
    fn and_binds_tighter_than_or() {
        let fields = [flag("a"), flag("b"), flag("c"), flag("d")];
        let clauses = vec![
            clause(0, JoinOp::And, "a", CmpOp::Eq, "1"),
            clause(1, JoinOp::And, "b", CmpOp::Eq, "2"),
            clause(2, JoinOp::Or, "c", CmpOp::Eq, "3"),
            clause(3, JoinOp::And, "d", CmpOp::Eq, "4"),
        ];
        let applied = apply_filters(DatabaseBackend::Postgres, &fields, &clauses);
        let rendered = sql(applied.condition);
        assert!(
            rendered.contains(r#"("a" = '1') AND ("b" = '2')"#)
                || rendered.contains(r#""a" = '1' AND "b" = '2'"#),
            "{rendered}"
        );
        assert!(rendered.contains(" OR "), "{rendered}");
        assert!(
            rendered.contains(r#"("c" = '3') AND ("d" = '4')"#)
                || rendered.contains(r#""c" = '3' AND "d" = '4'"#),
            "{rendered}"
        );
        let or_at = rendered.find(" OR ").expect("or");
        let c_at = rendered.find(r#""c""#).expect("c");
        assert!(or_at < c_at, "{rendered}");
    }

    #[test]
    fn first_join_is_ignored() {
        let fields = [flag("a"), flag("b")];
        let clauses = vec![
            clause(0, JoinOp::Or, "a", CmpOp::Eq, "1"),
            clause(1, JoinOp::And, "b", CmpOp::Eq, "2"),
        ];
        let rendered = sql(apply_filters(DatabaseBackend::Postgres, &fields, &clauses).condition);
        assert!(!rendered.contains(" OR "), "{rendered}");
        assert!(rendered.contains(" AND "), "{rendered}");
    }

    #[test]
    fn unknown_field_is_ignored_and_bad_number_is_an_error() {
        let fields = [number_column("priority", "Priority", NameCol)];
        let clauses = vec![
            clause(0, JoinOp::And, "missing", CmpOp::Eq, "x"),
            clause(1, JoinOp::And, "priority", CmpOp::Gt, "nope"),
        ];
        let applied = apply_filters(DatabaseBackend::Postgres, &fields, &clauses);
        assert!(applied.errors.iter().any(|err| err.index == 1));
        let rendered = sql(applied.condition);
        assert!(!rendered.contains("nope"), "{rendered}");
        assert!(!rendered.contains("missing"), "{rendered}");
    }

    #[test]
    fn text_match_does_not_order_rows() {
        let fields = [text_column("name", "Name", NameCol)];
        let clauses = vec![clause(0, JoinOp::And, "name", CmpOp::Eq, "Riv")];
        let applied = apply_filters(DatabaseBackend::Postgres, &fields, &clauses);
        let rendered = sql(applied.condition);
        assert!(rendered.contains("word_similarity"), "{rendered}");
        assert!(
            !rendered.to_ascii_lowercase().contains("order by"),
            "{rendered}"
        );

        let sqlite = Query::select()
            .column(Alias::new("id"))
            .from(Alias::new("t"))
            .cond_where(apply_filters(DatabaseBackend::Sqlite, &fields, &clauses).condition)
            .to_string(SqliteQueryBuilder);
        assert!(!sqlite.contains("word_similarity"), "{sqlite}");
        assert!(sqlite.to_ascii_lowercase().contains("like"), "{sqlite}");
        assert!(
            !sqlite.to_ascii_lowercase().contains("order by"),
            "{sqlite}"
        );
    }

    #[test]
    fn number_ops_bind_lt_gt_eq() {
        let fields = [number_column("n", "N", NameCol)];
        let gt = sql(apply_filters(
            DatabaseBackend::Postgres,
            &fields,
            &[clause(0, JoinOp::And, "n", CmpOp::Gt, "3")],
        )
        .condition);
        assert!(gt.contains('>'), "{gt}");
        let lt = sql(apply_filters(
            DatabaseBackend::Postgres,
            &fields,
            &[clause(0, JoinOp::And, "n", CmpOp::Lt, "3")],
        )
        .condition);
        assert!(lt.contains('<'), "{lt}");
        let eq = sql(apply_filters(
            DatabaseBackend::Postgres,
            &fields,
            &[clause(0, JoinOp::And, "n", CmpOp::Eq, "3")],
        )
        .condition);
        assert!(eq.contains('='), "{eq}");
        assert!(!eq.contains('>'), "{eq}");
        assert!(!eq.contains('<'), "{eq}");
    }
}
