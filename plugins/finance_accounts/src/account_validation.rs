use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QuerySelect, TransactionTrait,
};

use crate::{
    balance_type::BalanceType,
    entities::account::{self, Entity as AccountEntity},
};

/// Query param that scopes account pickers to one balance type (not a model field).
pub const BALANCE_TYPE_SCOPE_QUERY_PARAM: &str = "balance_type_scope";

/// Synthetic row id for the ".." parent directory entry in account pickers.
pub const ACCOUNT_PARENT_UP_ROW_ID: i64 = 0;

/// Returns the account picker URL filtered to the given balance type.
pub fn account_select_route_url(balance_type: BalanceType) -> String {
    lariv_core::http::RouteQueryBuilder::new(crate::routes::AccountSelectRouteTag)
        .query(BALANCE_TYPE_SCOPE_QUERY_PARAM, balance_type.as_str())
        .build()
}

/// Ensures `account_id` is a non-group account with the expected balance type.
pub async fn validate_leaf_account_balance_type(
    db: &DatabaseConnection,
    account_id: i64,
    want: BalanceType,
    label: &str,
) -> Result<(), String> {
    if account_id == 0 {
        return Err(format!("{label} is required"));
    }
    let acct = AccountEntity::find_by_id(account_id)
        .select_only()
        .column(account::Column::Id)
        .column(account::Column::BalanceType)
        .column(account::Column::IsGroup)
        .into_model::<AccountBalanceCheck>()
        .one(db)
        .await
        .map_err(|e| format!("{label}: {e}"))?
        .ok_or_else(|| format!("{label}: account not found"))?;
    if acct.is_group {
        return Err(format!(
            "{label}: group accounts cannot be used for posting"
        ));
    }
    if acct.balance_type != want {
        return Err(format!(
            "{label}: account must have balance type {}",
            want.as_str()
        ));
    }
    Ok(())
}

#[derive(Debug, sea_orm::FromQueryResult)]
struct AccountBalanceCheck {
    #[allow(dead_code)]
    id: i64,
    balance_type: BalanceType,
    is_group: bool,
}

/// Blocks assigning a parent that is this account or any of its descendants.
pub async fn validate_parent_not_cycle(
    db: &DatabaseConnection,
    account_id: Option<i64>,
    parent_id: Option<i64>,
) -> Result<(), String> {
    let Some(pid) = parent_id.filter(|&id| id > 0) else {
        return Ok(());
    };
    let Some(aid) = account_id.filter(|&id| id > 0) else {
        return Ok(());
    };
    if pid == aid {
        return Err("account cannot be its own parent".into());
    }
    let descendants = account_descendant_ids(db, aid)
        .await
        .map_err(|e| e.to_string())?;
    if descendants.contains(&pid) {
        return Err("parent cannot be a direct or indirect child of this account".into());
    }
    Ok(())
}

/// BFS descendant account ids including root. Parents appear before their children.
pub async fn account_descendant_ids<C: ConnectionTrait>(
    db: &C,
    root_id: i64,
) -> Result<Vec<i64>, sea_orm::DbErr> {
    let mut out = Vec::new();
    let mut queue = vec![root_id];
    let mut seen = std::collections::HashSet::new();
    seen.insert(root_id);
    while let Some(cur) = queue.pop() {
        out.push(cur);
        let kids: Vec<i64> = AccountEntity::find()
            .filter(account::Column::ParentId.eq(cur))
            .select_only()
            .column(account::Column::Id)
            .into_tuple()
            .all(db)
            .await?;
        for k in kids {
            if seen.insert(k) {
                queue.push(k);
            }
        }
    }
    Ok(out)
}

/// Delete `root_id` and every direct or indirect child. Children are removed
/// before their parents so the parent foreign key does not reparent them.
pub async fn delete_account_recursive(
    db: &DatabaseConnection,
    root_id: i64,
) -> Result<(), sea_orm::DbErr> {
    let txn = db.begin().await?;
    let ids = account_descendant_ids(&txn, root_id).await?;
    for id in ids.into_iter().rev() {
        AccountEntity::delete_by_id(id).exec(&txn).await?;
    }
    txn.commit().await?;
    Ok(())
}

/// Replace direct children of `parent_id` with `child_ids` (edit form sub-account list).
pub async fn sync_account_children(
    db: &DatabaseConnection,
    parent_id: i64,
    child_ids: &[i64],
) -> Result<(), String> {
    let mut want: std::collections::HashSet<i64> = child_ids
        .iter()
        .copied()
        .filter(|&id| id > 0 && id != parent_id)
        .collect();

    let current: Vec<i64> = AccountEntity::find()
        .filter(account::Column::ParentId.eq(parent_id))
        .select_only()
        .column(account::Column::Id)
        .into_tuple()
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    for cid in current.iter().filter(|id| !want.contains(id)) {
        let Some(child) = AccountEntity::find_by_id(*cid)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
        else {
            continue;
        };
        let model = account::ActiveModel {
            id: Set(child.id),
            parent_id: Set(None),
            ..Default::default()
        };
        model.update(db).await.map_err(|e| e.to_string())?;
    }

    for cid in want.drain() {
        validate_parent_not_cycle(db, Some(cid), Some(parent_id)).await?;
        let Some(child) = AccountEntity::find_by_id(cid)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
        else {
            return Err(format!("sub-account {cid} not found"));
        };
        let model = account::ActiveModel {
            id: Set(child.id),
            parent_id: Set(Some(parent_id)),
            ..Default::default()
        };
        model.update(db).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}
