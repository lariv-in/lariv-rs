//! Axum [`Query`] deserialization helpers and typed query URL patching.

use std::fmt;
use std::str::FromStr;

use axum::extract::Query;
use axum::http::Uri;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};

use crate::http::{RouteQueryBuilder, RouteUrl};

/// Rebuild a route URL after deserializing, mutating, and re-serializing query params.
pub fn patch_query_url<Q, R>(path_and_query: &str, route: R, patch: impl FnOnce(&mut Q)) -> String
where
    Q: DeserializeOwned + Default + ApplyQuery,
    R: RouteUrl,
{
    let uri = path_and_query_uri(path_and_query);
    let mut q = Query::<Q>::try_from_uri(&uri)
        .map(|Query(q)| q)
        .unwrap_or_default();
    patch(&mut q);
    q.apply_to(RouteQueryBuilder::new(route)).build()
}

fn path_and_query_uri(path_and_query: &str) -> Uri {
    let s = if path_and_query.starts_with('/') {
        format!("http://local{path_and_query}")
    } else {
        format!("http://local/{path_and_query}")
    };
    s.parse()
        .unwrap_or_else(|_| "http://local/".parse().unwrap())
}

/// Serialize a typed query struct into [`RouteQueryBuilder`] pairs.
pub trait ApplyQuery {
    fn apply_to<R: RouteUrl>(&self, builder: RouteQueryBuilder<R>) -> RouteQueryBuilder<R>;
}

/// Deserialize an optional query-string integer (works under `#[serde(flatten)]`).
pub fn query_u32<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    query_option::<D, u32>(deserializer)
}

/// Deserialize an optional query-string i64 (works under `#[serde(flatten)]`).
pub fn query_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    query_option::<D, i64>(deserializer)
}

/// Deserialize an optional query-string bool (`on` / `true` / `1`, etc.).
pub fn query_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    match s.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some("on" | "true" | "1") => Ok(Some(true)),
        Some("false" | "0") => Ok(Some(false)),
        Some(other) => Err(serde::de::Error::custom(format!(
            "invalid bool query param: {other:?}"
        ))),
    }
}

/// Deserialize an optional query-string text field. Empty/absent → `None`.
pub fn query_str<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    Ok(match s.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(s) => Some(s.to_owned()),
    })
}

fn query_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    let s = Option::<String>::deserialize(deserializer)?;
    match s.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(s) => T::from_str(s).map(Some).map_err(serde::de::Error::custom),
    }
}

/// Pagination index from a query string (`page=2`). Defaults to `1`.
///
/// Use on list filters and FK picker routes instead of raw `Option<u32>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QueryPage(Option<u32>);

impl QueryPage {
    /// Resolved page number (minimum 1).
    pub fn get(self) -> u32 {
        self.0.unwrap_or(1).max(1)
    }

    /// Raw optional value from the query string.
    pub fn raw(self) -> Option<u32> {
        self.0
    }

    pub fn set(&mut self, page: Option<u32>) {
        self.0 = page;
    }
}

impl<'de> Deserialize<'de> for QueryPage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        query_u32(deserializer).map(Self)
    }
}

/// Rows-per-page from a query string (`page_size=36`). Defaults to [`DEFAULT_PAGE_SIZE`].
///
/// Values outside [`PAGE_SIZE_CHOICES`] fall back to the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QueryPageSize(Option<u32>);

impl QueryPageSize {
    /// Resolved page size (clamped to allowed choices).
    pub fn get(self) -> u32 {
        crate::components::clamp_page_size(self.0)
    }

    /// Raw optional value from the query string.
    pub fn raw(self) -> Option<u32> {
        self.0
    }

    pub fn set(&mut self, page_size: Option<u32>) {
        self.0 = page_size;
    }
}

impl<'de> Deserialize<'de> for QueryPageSize {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        query_u32(deserializer).map(Self)
    }
}

/// Optional i64 from a query string (`ParentID=`, `ParentID=5`). Empty/absent → `None`.
///
/// Use on axum [`Query`] structs for FK picker drill-down and exclude-id parameters
/// instead of raw `Option<i64>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QueryI64(Option<i64>);

impl QueryI64 {
    /// Raw optional value from the query string.
    pub fn get(self) -> Option<i64> {
        self.0
    }

    /// `Some(id)` only when `id > 0`.
    pub fn positive(self) -> Option<i64> {
        self.0.filter(|&id| id > 0)
    }

    /// Resolved id, defaulting to `0` when absent.
    pub fn or_zero(self) -> i64 {
        self.0.unwrap_or(0)
    }

    pub fn set(&mut self, value: Option<i64>) {
        self.0 = value;
    }
}

impl<'de> Deserialize<'de> for QueryI64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        query_i64(deserializer).map(Self)
    }
}

/// Optional text from a query string (`Name=`, `Name=foo`). Empty/absent → `None`.
///
/// Use on axum [`Query`] list-filter structs instead of raw `Option<String>`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueryStr(Option<String>);

impl QueryStr {
    /// Raw optional value from the query string.
    pub fn get(self) -> Option<String> {
        self.0
    }

    /// Borrowed filter value when non-empty.
    pub fn as_deref(&self) -> Option<&str> {
        self.0.as_deref()
    }

    /// Resolved value for repopulating filter forms.
    pub fn or_empty(self) -> String {
        self.0.unwrap_or_default()
    }
}

impl<'de> Deserialize<'de> for QueryStr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        query_str(deserializer).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use axum::extract::Query;
    use axum::http::Uri;
    use serde::Deserialize;

    use super::{ApplyQuery, QueryI64, QueryPage, QueryStr, patch_query_url};
    use crate::http::{RouteQueryBuilder, RouteTag, RouteUrl};

    #[derive(Debug, Deserialize, Default)]
    struct FlatFilters {
        #[serde(default)]
        page: QueryPage,
        #[serde(default, rename = "Name")]
        name: QueryStr,
    }

    #[derive(Debug, Deserialize, Default)]
    struct FlatPickerQuery {
        #[serde(flatten)]
        filter: FlatFilters,
        #[serde(default)]
        target_input: Option<String>,
    }

    impl ApplyQuery for FlatPickerQuery {
        fn apply_to<R: crate::http::RouteUrl>(
            &self,
            builder: RouteQueryBuilder<R>,
        ) -> RouteQueryBuilder<R> {
            let mut b = builder;
            if let Some(name) = self.filter.name.as_deref() {
                b = b.query("Name", name);
            }
            if let Some(page) = self.filter.page.raw() {
                b = b.query("page", page);
            }
            if let Some(target) = self.target_input.as_deref() {
                b = b.query("target_input", target);
            }
            b
        }
    }

    #[derive(Debug, Deserialize, Default)]
    struct AccountPickerQuery {
        #[serde(default, rename = "ParentID", alias = "parent_id")]
        parent_id: QueryI64,
        #[serde(default)]
        exclude_account_id: QueryI64,
        #[serde(default)]
        target_input: Option<String>,
    }

    #[test]
    fn query_page_deserializes_under_flatten() {
        let uri: Uri = "/finance/currencies/select?target_input=CurrencyID&page=2"
            .parse()
            .unwrap();
        let Query(q) = Query::<FlatPickerQuery>::try_from_uri(&uri).unwrap();
        assert_eq!(q.filter.page.get(), 2);
        assert_eq!(q.target_input.as_deref(), Some("CurrencyID"));
    }

    #[test]
    fn query_page_defaults_to_one() {
        let uri: Uri = "/items/select".parse().unwrap();
        let Query(q) = Query::<FlatPickerQuery>::try_from_uri(&uri).unwrap();
        assert_eq!(q.filter.page.get(), 1);
    }

    #[test]
    fn query_page_rejects_invalid() {
        let uri: Uri = "/items/select?page=abc".parse().unwrap();
        assert!(Query::<FlatPickerQuery>::try_from_uri(&uri).is_err());
    }

    #[test]
    fn query_str_deserializes_empty_name() {
        let uri: Uri = "/finance/accounts/select?Name=&Code=&BalanceType="
            .parse()
            .unwrap();
        let Query(q) = Query::<FlatPickerQuery>::try_from_uri(&uri).unwrap();
        assert_eq!(q.filter.name.as_deref(), None);
    }

    #[test]
    fn query_str_deserializes_nonempty_name() {
        let uri: Uri = "/finance/accounts/select?Name=SGST".parse().unwrap();
        let Query(q) = Query::<FlatPickerQuery>::try_from_uri(&uri).unwrap();
        assert_eq!(q.filter.name.as_deref(), Some("SGST"));
    }

    #[test]
    fn query_i64_deserializes_empty_parent_id() {
        let uri: Uri = "/finance/accounts/select?ParentID=&target_input=AccountID&Name=foo"
            .parse()
            .unwrap();
        let Query(q) = Query::<AccountPickerQuery>::try_from_uri(&uri).unwrap();
        assert_eq!(q.parent_id.get(), None);
        assert_eq!(q.exclude_account_id.get(), None);
        assert_eq!(q.target_input.as_deref(), Some("AccountID"));
    }

    #[test]
    fn query_i64_deserializes_numeric_parent_id() {
        let uri: Uri = "/finance/accounts/select?ParentID=42&exclude_account_id=7"
            .parse()
            .unwrap();
        let Query(q) = Query::<AccountPickerQuery>::try_from_uri(&uri).unwrap();
        assert_eq!(q.parent_id.positive(), Some(42));
        assert_eq!(q.exclude_account_id.positive(), Some(7));
    }

    #[test]
    fn patch_query_url_updates_page() {
        struct DummyRoute;
        impl crate::http::RouteTag for DummyRoute {
            const PATH: &'static str = "/items/select";
        }
        impl crate::http::RouteUrl for DummyRoute {
            fn path(self) -> String {
                Self::PATH.to_string()
            }
            fn url(self) -> String {
                Self::PATH.to_string()
            }
        }

        let url = patch_query_url::<FlatPickerQuery, _>(
            "/items/select?target_input=CurrencyID&page=2",
            DummyRoute,
            |q| q.filter.page.set(Some(1)),
        );
        assert!(url.contains("page=1"), "{url}");
        assert!(url.contains("target_input=CurrencyID"), "{url}");
    }
}
