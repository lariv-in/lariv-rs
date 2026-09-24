use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, JoinType, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait, Select, sea_query::Expr,
};

use crate::plugins::users::{
    entities::user::{self, Entity as UserEntity},
    state::AuthContext,
};

use super::entities::{
    task::{self, Entity as TaskEntity},
    task_log::{self, Entity as TaskLogEntity},
    task_status::{self, Entity as TaskStatusEntity},
};

pub fn scope_superuser<T>(query: Select<T>, auth: &AuthContext) -> Select<T>
where
    T: EntityTrait,
{
    if auth.user.is_superuser {
        return query;
    }
    query.filter(Expr::cust("1 = 0"))
}

pub async fn find_task_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<task::Model> {
    crate::web::opt_or_log(
        scope_superuser(TaskEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find by id",
    )
}

pub async fn find_status_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<task_status::Model> {
    crate::web::opt_or_log(
        scope_superuser(TaskStatusEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find by id",
    )
}

pub async fn find_log_scoped(
    db: &DatabaseConnection,
    id: i64,
    auth: &AuthContext,
) -> Option<task_log::Model> {
    crate::web::opt_or_log(
        scope_superuser(TaskLogEntity::find_by_id(id), auth)
            .one(db)
            .await,
        "find by id",
    )
}

fn sort_key(sort: &str) -> &str {
    sort.split_whitespace().next().unwrap_or("")
}

fn sort_desc(sort: &str) -> bool {
    sort.split_whitespace()
        .last()
        .is_some_and(|d| d.eq_ignore_ascii_case("DESC"))
}

pub fn apply_task_filters(
    mut query: Select<TaskEntity>,
    title: Option<&str>,
    assigned_to_id: Option<i64>,
    status_id: Option<i64>,
) -> Select<TaskEntity> {
    if let Some(t) = title.filter(|s| !s.is_empty()) {
        query = query.filter(task::Column::Title.contains(t));
    }
    if let Some(uid) = assigned_to_id.filter(|id| *id > 0) {
        query = query.filter(task::Column::AssignedToId.eq(uid));
    }
    if let Some(sid) = status_id.filter(|id| *id > 0) {
        query = query.filter(task::Column::StatusId.eq(sid));
    }
    query
}

pub fn apply_task_sort(mut query: Select<TaskEntity>, sort: Option<&str>) -> Select<TaskEntity> {
    let sort = sort.unwrap_or("").trim();
    let key = sort_key(sort);
    if key.eq_ignore_ascii_case("AssignedTo") {
        query = query.join(JoinType::LeftJoin, task::Relation::AssignedTo.def());
    }
    if key.eq_ignore_ascii_case("Status") {
        query = query.join(JoinType::LeftJoin, task::Relation::Status.def());
    }
    let desc = sort_desc(sort);
    match key {
        s if s.eq_ignore_ascii_case("Title") => {
            if desc {
                query.order_by_desc(task::Column::Title)
            } else {
                query.order_by_asc(task::Column::Title)
            }
        }
        s if s.eq_ignore_ascii_case("AssignedTo") => {
            if desc {
                query.order_by_desc(user::Column::Name)
            } else {
                query.order_by_asc(user::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("Status") => {
            if desc {
                query.order_by_desc(task_status::Column::Name)
            } else {
                query.order_by_asc(task_status::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("Priority") => {
            if desc {
                query.order_by_desc(task::Column::Priority)
            } else {
                query.order_by_asc(task::Column::Priority)
            }
        }
        s if s.eq_ignore_ascii_case("DueDatetime") => {
            if desc {
                query.order_by_desc(task::Column::DueDatetime)
            } else {
                query.order_by_asc(task::Column::DueDatetime)
            }
        }
        _ => query.order_by_desc(task::Column::Id),
    }
}

pub fn apply_status_sort(
    query: Select<TaskStatusEntity>,
    sort: Option<&str>,
) -> Select<TaskStatusEntity> {
    let sort = sort.unwrap_or("").trim();
    let desc = sort_desc(sort);
    match sort_key(sort) {
        s if s.eq_ignore_ascii_case("Name") => {
            if desc {
                query.order_by_desc(task_status::Column::Name)
            } else {
                query.order_by_asc(task_status::Column::Name)
            }
        }
        _ => query.order_by_desc(task_status::Column::Id),
    }
}

pub async fn user_exists(db: &DatabaseConnection, id: i64) -> bool {
    if id <= 0 {
        return false;
    }
    crate::web::opt_or_log(UserEntity::find_by_id(id).one(db).await, "find user by id").is_some()
}

pub async fn user_display_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    crate::web::opt_or_log(UserEntity::find_by_id(id).one(db).await, "find user by id")
        .map(|u| u.name)
        .unwrap_or_default()
}

pub async fn status_exists(db: &DatabaseConnection, id: i64) -> bool {
    if id <= 0 {
        return false;
    }
    crate::web::opt_or_log(
        TaskStatusEntity::find_by_id(id).one(db).await,
        "find status by id",
    )
    .is_some()
}

pub async fn status_display_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    crate::web::opt_or_log(
        TaskStatusEntity::find_by_id(id).one(db).await,
        "find status by id",
    )
    .map(|s| s.name)
    .unwrap_or_default()
}

pub async fn load_status_choices(db: &DatabaseConnection) -> Vec<(String, String)> {
    let models = TaskStatusEntity::find()
        .order_by_asc(task_status::Column::Name)
        .all(db)
        .await
        .unwrap_or_default();
    models
        .into_iter()
        .map(|s| (s.id.to_string(), s.name))
        .collect()
}

pub async fn load_status_map(
    db: &DatabaseConnection,
) -> std::collections::HashMap<i64, (String, u32)> {
    let models = TaskStatusEntity::find().all(db).await.unwrap_or_default();
    models
        .into_iter()
        .map(|s| (s.id, (s.name, s.color)))
        .collect()
}
