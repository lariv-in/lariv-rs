//! Checked fields for a git sync row.

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use lariv_plugin_filesystem::node;

use crate::entities::filesystem_git_sync::{Column, Entity};
use crate::forms::GitSyncForm;

pub struct SyncFields {
    pub git_url: String,
    pub git_identifier: String,
    pub subpath: String,
    pub directory_vnode_id: i64,
}

/// Git Sync merges only when the resolved commit differs from the last successful sync.
/// Force Sync always merges.
pub fn should_merge(force: bool, last_synced: Option<&str>, resolved: &str) -> bool {
    force || last_synced.is_none_or(|previous| previous.is_empty() || previous != resolved)
}

pub fn normalize_subpath(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(String::new());
    }
    if raw.starts_with('/') || raw.contains('\\') {
        return Err("subpath must be a relative path".into());
    }
    let mut parts = Vec::new();
    for segment in raw.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return Err("subpath cannot contain ..".into());
        }
        parts.push(segment);
    }
    Ok(parts.join("/"))
}

pub fn normalize_identifier(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("git identifier is required".into());
    }
    if let Ok(oid) = gix::ObjectId::from_hex(raw.as_bytes()) {
        return Ok(oid.to_hex().to_string());
    }
    if !is_ref_name(raw) {
        return Err("git identifier must be a branch, tag, or full commit id".into());
    }
    Ok(raw.to_string())
}

fn is_ref_name(raw: &str) -> bool {
    if raw.chars().any(char::is_whitespace)
        || raw.starts_with('/')
        || raw.ends_with('/')
        || raw.ends_with('.')
        || raw.contains('\\')
        || raw.contains("..")
        || raw.contains("//")
        || raw.contains("@{")
    {
        return false;
    }
    let forbidden = ['~', '^', ':', '?', '*', '[', ' '];
    if raw.contains(forbidden) {
        return false;
    }
    raw.split('/')
        .all(|part| !part.is_empty() && !part.starts_with('.') && !part.ends_with(".lock"))
}

pub async fn directory_label(db: &DatabaseConnection, raw: &str) -> String {
    let Ok(id) = raw.trim().parse::<i64>() else {
        return String::new();
    };
    match node::get_by_id(db, id).await {
        Ok(Some(node)) => node.name,
        _ => String::new(),
    }
}

pub async fn parse_sync_form(
    db: &DatabaseConnection,
    form: &GitSyncForm,
    exclude_id: Option<i64>,
) -> Result<SyncFields, String> {
    let git_url = form.git_url.trim();
    if git_url.is_empty() {
        return Err("git url is required".into());
    }
    if git_url.chars().any(char::is_whitespace) {
        return Err("git url cannot contain whitespace".into());
    }
    let git_identifier = normalize_identifier(&form.git_identifier)?;
    let subpath = normalize_subpath(&form.subpath)?;
    let directory_vnode_id = form
        .directory_vnode_id
        .trim()
        .parse::<i64>()
        .map_err(|error| format!("select a directory ({error})"))?;
    let Some(directory) = node::get_by_id(db, directory_vnode_id)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Err("directory was not found".into());
    };
    if !directory.is_directory {
        return Err("select a directory".into());
    }
    let existing = Entity::find()
        .filter(Column::DirectoryVnodeId.eq(directory_vnode_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    if existing.is_some_and(|row| exclude_id.is_none_or(|id| row.directory_vnode_id != id)) {
        return Err("this directory already has a git sync".into());
    }
    Ok(SyncFields {
        git_url: git_url.to_string(),
        git_identifier,
        subpath,
        directory_vnode_id,
    })
}

#[cfg(test)]
mod tests {
    use super::{normalize_identifier, should_merge};

    #[test]
    fn identifier_accepts_a_commit_branch_or_tag() {
        let oid = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(normalize_identifier(oid).expect("oid"), oid);
        assert_eq!(normalize_identifier("main").expect("branch"), "main");
        assert_eq!(
            normalize_identifier("refs/tags/v1").expect("tag"),
            "refs/tags/v1"
        );
        assert!(normalize_identifier("refs/heads/../main").is_err());
        assert!(normalize_identifier("/tmp/repo").is_err());
    }

    #[test]
    fn git_sync_skips_only_when_the_commit_matches() {
        let resolved = "abc";
        assert!(!should_merge(false, Some(resolved), resolved));
        assert!(should_merge(true, Some(resolved), resolved));
        assert!(should_merge(false, Some("other"), resolved));
        assert!(should_merge(false, None, resolved));
    }
}
