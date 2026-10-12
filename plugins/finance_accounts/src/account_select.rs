use crate::{
    account_validation::{ACCOUNT_PARENT_UP_ROW_ID, BALANCE_TYPE_SCOPE_QUERY_PARAM},
    forms::AccountFormField,
    routes::AccountSelectRouteTag,
};
use lariv_core::components::{
    HX_TARGET_CLOSEST_TABLE,
    attrs::HtmlAttrs,
    htmx::{row_attr_select_extra, row_attr_select_multi},
};
use lariv_core::html_form::FormFieldKey;
use lariv_core::http::RouteQueryBuilder;

/// Browse scope for the chart and the account picker.
///
/// With no filter, the chart shows roots and a drilled-in picker shows that
/// parent's children. A name, code, balance type, or group filter searches the
/// whole tree so nested accounts are not hidden by the current level.
pub fn account_browse_scope(parent_id: Option<i64>, filters_active: bool) -> (Option<i64>, bool) {
    if filters_active {
        return (None, false);
    }
    match parent_id.filter(|&id| id > 0) {
        Some(id) => (Some(id), false),
        None => (None, true),
    }
}

/// Keep every query pair except parent and page, then set the drill parent and page 1.
pub fn rewrite_account_select_query(path_and_query: &str, parent_id: Option<i64>) -> String {
    let (path, query) = path_and_query
        .split_once('?')
        .map(|(path, query)| (path, query))
        .unwrap_or((path_and_query, ""));
    let mut kept = Vec::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let key = pair.split_once('=').map(|(key, _)| key).unwrap_or(pair);
        if key == "ParentID" || key == "parent_id" || key == "page" {
            continue;
        }
        kept.push(pair.to_string());
    }
    if let Some(id) = parent_id.filter(|id| *id > 0) {
        kept.push(format!("ParentID={id}"));
    }
    kept.push("page=1".into());
    format!("{path}?{}", kept.join("&"))
}

fn account_select_browse_attrs(url: &str) -> HtmlAttrs {
    HtmlAttrs::new()
        .set(
            "class",
            "cursor-pointer hover:bg-base-200 transition-colors",
        )
        .set("hx-get", url)
        .set("hx-target", HX_TARGET_CLOSEST_TABLE)
        .set("hx-swap", "outerHTML")
        .set("hx-push-url", "false")
}

pub fn account_select_drill_url(path_and_query: &str, parent_id: i64) -> String {
    rewrite_account_select_query(path_and_query, Some(parent_id))
}

pub fn account_select_parent_up_url(
    path_and_query: &str,
    grandparent_id: Option<i64>,
) -> Option<String> {
    Some(rewrite_account_select_query(path_and_query, grandparent_id))
}

pub fn account_selection_row_attrs(
    row_id: i64,
    is_group: bool,
    balance_type: &str,
    target_input: &str,
    display: &str,
    path_and_query: &str,
    parent_up_url: Option<&str>,
    drill_parent_id: i64,
) -> HtmlAttrs {
    if row_id == ACCOUNT_PARENT_UP_ROW_ID {
        if let Some(url) = parent_up_url {
            return account_select_browse_attrs(url);
        }
    }
    let parent_picker = target_input == AccountFormField::ParentId.target_input();
    let child_picker = target_input == AccountFormField::ChildIds.target_input();
    if is_group && !parent_picker && !child_picker {
        let url = account_select_drill_url(path_and_query, drill_parent_id);
        return account_select_browse_attrs(&url);
    }
    if child_picker {
        return row_attr_select_multi(target_input, &row_id.to_string(), display);
    }
    row_attr_select_extra(
        target_input,
        &row_id.to_string(),
        display,
        &[("balance_type", balance_type)],
    )
}

/// HTMX attrs for the Open button that lists a group account's children.
pub fn account_selection_drill_attrs(path_and_query: &str, drill_parent_id: i64) -> HtmlAttrs {
    let url = account_select_drill_url(path_and_query, drill_parent_id);
    HtmlAttrs::new()
        .set("type", "button")
        .set("class", "btn btn-ghost btn-xs")
        .set("hx-get", url)
        .set("hx-target", HX_TARGET_CLOSEST_TABLE)
        .set("hx-swap", "outerHTML")
        .set("hx-push-url", "false")
        .set("@click.stop", "")
}

pub fn account_select_url_with_balance_type(balance_type: &str) -> String {
    format!(
        "{}?{}={balance_type}",
        AccountSelectRouteTag.url(),
        BALANCE_TYPE_SCOPE_QUERY_PARAM,
    )
}

/// Sub-account picker for the account currently being edited: list its children
/// and exclude itself so create-from-picker prefills that parent.
pub fn account_select_children_url(account_id: i64) -> String {
    RouteQueryBuilder::new(AccountSelectRouteTag)
        .query("ParentID", account_id)
        .query("exclude_account_id", account_id)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browse_scope_roots_until_a_filter_or_parent() {
        assert_eq!(account_browse_scope(None, false), (None, true));
        assert_eq!(account_browse_scope(Some(3), false), (Some(3), false));
        assert_eq!(account_browse_scope(None, true), (None, false));
        assert_eq!(account_browse_scope(Some(3), true), (None, false));
    }

    #[test]
    fn drill_url_sets_parent_and_resets_page() {
        let url = account_select_drill_url(
            "/finance/accounts/select/?target_input=AccountID&page=2",
            42,
        );
        assert!(url.contains("ParentID=42"), "{url}");
        assert!(url.contains("target_input=AccountID"), "{url}");
        assert!(url.contains("page=1"), "{url}");
    }

    #[test]
    fn parent_up_url_clears_parent_at_root() {
        let url = account_select_parent_up_url(
            "/finance/accounts/select/?ParentID=42&target_input=AccountID",
            None,
        )
        .expect("up url");
        assert!(!url.contains("ParentID="), "{url}");
        assert!(url.contains("target_input=AccountID"), "{url}");
    }

    #[test]
    fn drill_button_opens_child_listing_in_table() {
        let html = account_selection_drill_attrs("/finance/accounts/select/", 7).as_string();
        assert!(html.contains("hx-get"), "{html}");
        assert!(html.contains("ParentID=7"), "{html}");
        assert!(html.contains(HX_TARGET_CLOSEST_TABLE), "{html}");
        assert!(html.contains("@click.stop"), "{html}");
    }

    #[test]
    fn group_row_in_posting_picker_drills_instead_of_select() {
        let html = account_selection_row_attrs(
            7,
            true,
            "Debit",
            "AccountID",
            "1000 — Assets",
            "/finance/accounts/select/?target_input=AccountID",
            None,
            7,
        )
        .as_string();
        assert!(html.contains("hx-get"), "{html}");
        assert!(html.contains("ParentID=7"), "{html}");
        assert!(!html.contains("fk-select"), "{html}");
    }

    #[test]
    fn group_row_in_parent_picker_selects() {
        let html = account_selection_row_attrs(
            7,
            true,
            "Debit",
            AccountFormField::ParentId.target_input(),
            "1000 — Assets",
            "/finance/accounts/select/?target_input=ParentID",
            None,
            7,
        )
        .as_string();
        assert!(html.contains("fk-select"), "{html}");
        assert!(!html.contains("hx-get"), "{html}");
    }

    #[test]
    fn children_picker_url_opens_on_the_current_account() {
        let url = account_select_children_url(7);
        assert!(url.contains("ParentID=7"), "{url}");
        assert!(url.contains("exclude_account_id=7"), "{url}");
    }
}
