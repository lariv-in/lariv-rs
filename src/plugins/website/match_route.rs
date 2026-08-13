//! / wildcard / ltree helpers.

use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter,
};

use super::entities::{
    DbRoute,
    db_route::{self, Entity as DbRouteEntity},
};

pub fn path_to_ltree(path: &str) -> String {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return "root".into();
    }
    trimmed.replace('/', ".").replace('-', "_")
}

pub fn path_to_lquery(pattern: &str) -> String {
    path_to_ltree(pattern)
}

/// .
pub fn match_go_wildcard(pattern: &str, req_path: &str) -> bool {
    let pattern = pattern.trim_matches('/');
    let req_path = req_path.trim_matches('/');

    if pattern == req_path || pattern == "*" {
        return true;
    }

    let p_parts: Vec<&str> = if pattern.is_empty() {
        vec![]
    } else {
        pattern.split('/').collect()
    };
    let r_parts: Vec<&str> = if req_path.is_empty() {
        vec![]
    } else {
        req_path.split('/').collect()
    };

    if p_parts.len() != r_parts.len() {
        if let Some(last) = p_parts.last()
            && (*last == "*" || last.starts_with("*{"))
            && r_parts.len() >= p_parts.len().saturating_sub(1)
        {
            let prefix_ok = p_parts[..p_parts.len() - 1]
                .iter()
                .enumerate()
                .all(|(i, p)| *p == "*" || r_parts.get(i) == Some(p));
            if prefix_ok {
                return true;
            }
        }
        return false;
    }

    p_parts
        .iter()
        .zip(r_parts.iter())
        .all(|(p, r)| *p == "*" || p == r)
}

/// Find the first active [`DbRoute`] matching `req_path`.
pub async fn find_matching_db_route(
    db: &DatabaseConnection,
    req_path: &str,
) -> Result<Option<DbRoute>, DbErr> {
    let req_ltree = path_to_ltree(req_path);

    // 1. Exact path match
    if let Some(route) = DbRouteEntity::find()
        .filter(db_route::Column::Path.eq(req_path))
        .filter(db_route::Column::IsActive.eq(true))
        .one(db)
        .await?
    {
        return Ok(Some(route));
    }

    // 2. Exact ltree_path (Postgres only)
    if db.get_database_backend() == DatabaseBackend::Postgres {
        let rows = db
            .query_all(sea_orm::Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT id FROM db_routes WHERE is_active = TRUE AND ltree_path = $1::ltree LIMIT 1",
                [req_ltree.clone().into()],
            ))
            .await?;
        if let Some(row) = rows.first() {
            let id: i64 = row.try_get("", "id")?;
            if let Some(route) = DbRouteEntity::find_by_id(id).one(db).await? {
                return Ok(Some(route));
            }
        }
    }

    // 3. Active routes: lquery (Postgres) or wildcard
    let active = DbRouteEntity::find()
        .filter(db_route::Column::IsActive.eq(true))
        .all(db)
        .await?;

    for route in active {
        let mut matched = false;

        if db.get_database_backend() == DatabaseBackend::Postgres {
            let lquery = path_to_lquery(&route.path);
            let count = db
                .query_one(sea_orm::Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT COUNT(*) AS c FROM (SELECT $1::ltree AS t) sub WHERE t ~ $2::lquery",
                    [req_ltree.clone().into(), lquery.into()],
                ))
                .await?;
            if let Some(row) = count {
                let c: i64 = row.try_get("", "c").unwrap_or(0);
                if c > 0 {
                    matched = true;
                }
            }
        }

        if !matched {
            matched = match_go_wildcard(&route.path, req_path);
        }

        if matched {
            return Ok(Some(route));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_exact_and_star() {
        assert!(match_go_wildcard("/blog", "/blog"));
        assert!(match_go_wildcard("*", "/anything"));
        assert!(match_go_wildcard("/blog/*", "/blog/foo"));
        assert!(!match_go_wildcard("/blog/*", "/other/foo"));
    }

    #[test]
    fn path_to_ltree_root() {
        assert_eq!(path_to_ltree("/"), "root");
        assert_eq!(path_to_ltree("/a/b-c"), "a.b_c");
    }
}
