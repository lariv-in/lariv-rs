use crate::components::{
    HX_TARGET_CLOSEST_TABLE,
    attrs::HtmlAttrs,
    htmx::{row_attr_select_extra, row_attr_select_multi},
};
use crate::html_form::FormFieldKey;
use crate::plugins::finance_accounts::{
    account_validation::{ACCOUNT_PARENT_UP_ROW_ID, BALANCE_TYPE_SCOPE_QUERY_PARAM},
    forms::AccountFormField,
    handlers::accounts::AccountSelectQuery,
    routes::AccountSelectRouteTag,
};
use crate::web::patch_query_url;

/// Browse like the chart of accounts: roots only until a group is opened.
/// Name/code search flattens so typeahead can still match nested accounts.
pub fn account_select_root_only(
    parent_id: Option<i64>,
    name: Option<&str>,
    code: Option<&str>,
) -> bool {
    parent_id.is_none() && name.is_none() && code.is_none()
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
    patch_query_url::<AccountSelectQuery, _>(path_and_query, AccountSelectRouteTag, |q| {
        q.parent_id.set(Some(parent_id));
        q.filter.page.set(Some(1));
    })
}

pub fn account_select_parent_up_url(
    path_and_query: &str,
    grandparent_id: Option<i64>,
) -> Option<String> {
    Some(patch_query_url::<AccountSelectQuery, _>(
        path_and_query,
        AccountSelectRouteTag,
        |q| {
            q.parent_id.set(grandparent_id.filter(|&id| id > 0));
            q.filter.page.set(Some(1));
        },
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_only_when_browsing_top_level() {
        assert!(account_select_root_only(None, None, None));
        assert!(!account_select_root_only(Some(3), None, None));
        assert!(!account_select_root_only(None, Some("Cash"), None));
        assert!(!account_select_root_only(None, None, Some("1000")));
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
}
