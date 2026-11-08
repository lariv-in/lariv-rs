//! Fetch one commit with `gix` and read the tree at a subpath.

use gix::bstr::ByteSlice;

use crate::validate::normalize_subpath;

/// One name in a git tree, holding either file bytes or child entries.
pub struct GitEntry {
    pub name: String,
    pub kind: GitEntryKind,
}

pub enum GitEntryKind {
    File(Vec<u8>),
    Directory(Vec<GitEntry>),
}

fn err_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn read_tree(tree: &gix::Tree<'_>) -> Result<Vec<GitEntry>, String> {
    let mut entries = Vec::new();
    for entry in tree.iter() {
        let entry = entry.map_err(err_string)?;
        let name = entry
            .filename()
            .to_str()
            .map_err(|error| error.to_string())?
            .to_owned();
        let mode = entry.mode();
        if mode.is_commit() {
            return Err(format!("submodule {name} is not supported"));
        }
        if mode.is_tree() {
            let child = entry
                .object()
                .map_err(err_string)?
                .try_into_tree()
                .map_err(err_string)?;
            entries.push(GitEntry {
                name,
                kind: GitEntryKind::Directory(read_tree(&child)?),
            });
        } else {
            let blob = entry
                .object()
                .map_err(err_string)?
                .try_into_blob()
                .map_err(err_string)?;
            let data = blob.data.clone();
            entries.push(GitEntry {
                name,
                kind: GitEntryKind::File(data),
            });
        }
    }
    Ok(entries)
}

fn tree_at_subpath<'repo>(
    repo: &'repo gix::Repository,
    commit_id: gix::ObjectId,
    subpath: &str,
) -> Result<gix::Tree<'repo>, String> {
    let subpath = normalize_subpath(subpath)?;
    let commit = repo
        .find_object(commit_id)
        .map_err(err_string)?
        .try_into_commit()
        .map_err(err_string)?;
    let root = commit.tree().map_err(err_string)?;
    if subpath.is_empty() {
        return Ok(root);
    }
    let entry = root
        .lookup_entry_by_path(&subpath)
        .map_err(err_string)?
        .ok_or_else(|| format!("subpath {subpath} was not found"))?;
    if !entry.mode().is_tree() {
        return Err(format!("subpath {subpath} is not a directory"));
    }
    entry
        .object()
        .map_err(err_string)?
        .try_into_tree()
        .map_err(err_string)
}

/// Tree fetched from a resolved commit.
pub struct FetchedTree {
    pub commit_id: String,
    pub entries: Vec<GitEntry>,
}

fn refspecs(identifier: &str) -> Vec<String> {
    if gix::ObjectId::from_hex(identifier.as_bytes()).is_ok() {
        return vec![format!("+{identifier}:refs/lariv/sync")];
    }
    if let Some(name) = identifier.strip_prefix("refs/heads/") {
        return vec![format!("+refs/heads/{name}:refs/lariv/sync")];
    }
    if let Some(name) = identifier.strip_prefix("refs/tags/") {
        return vec![format!("+refs/tags/{name}:refs/lariv/sync")];
    }
    vec![
        format!("+refs/heads/{identifier}:refs/lariv/sync-head"),
        format!("+refs/tags/{identifier}:refs/lariv/sync-tag"),
    ]
}

fn peel_commit(repo: &gix::Repository, name: &str) -> Result<Option<gix::ObjectId>, String> {
    let mut reference = match repo.find_reference(name) {
        Ok(reference) => reference,
        Err(_) => return Ok(None),
    };
    let commit = reference.peel_to_commit().map_err(err_string)?;
    Ok(Some(commit.id().detach()))
}

fn resolve_fetched(repo: &gix::Repository, identifier: &str) -> Result<gix::ObjectId, String> {
    let direct = gix::ObjectId::from_hex(identifier.as_bytes()).is_ok()
        || identifier.starts_with("refs/heads/")
        || identifier.starts_with("refs/tags/");
    if direct {
        return peel_commit(repo, "refs/lariv/sync")?
            .ok_or_else(|| format!("git identifier {identifier} was not found"));
    }
    if let Some(commit_id) = peel_commit(repo, "refs/lariv/sync-head")? {
        return Ok(commit_id);
    }
    peel_commit(repo, "refs/lariv/sync-tag")?
        .ok_or_else(|| format!("git identifier {identifier} was not found"))
}

/// Fetch `identifier` from `url` into a temporary repository and return the tree at `subpath`.
///
/// A full object id is fetched directly. `refs/heads/…` and `refs/tags/…` fetch that ref.
/// Any other name fetches both, and a branch wins when both exist. Tags are peeled to a commit.
pub fn fetch_entries(url: &str, identifier: &str, subpath: &str) -> Result<FetchedTree, String> {
    let identifier = crate::validate::normalize_identifier(identifier)?;
    let dir = tempfile::tempdir().map_err(err_string)?;
    let repo = gix::init_bare(dir.path()).map_err(err_string)?;
    let specs = refspecs(&identifier);
    let remote = repo
        .remote_at(url)
        .map_err(err_string)?
        .with_refspecs(
            specs.iter().map(String::as_str),
            gix::remote::Direction::Fetch,
        )
        .map_err(err_string)?;
    let options = gix::remote::ref_map::Options {
        prefix_from_spec_as_filter_on_remote: false,
        ..Default::default()
    };
    remote
        .connect(gix::remote::Direction::Fetch)
        .map_err(err_string)?
        .prepare_fetch(gix::progress::Discard, options)
        .map_err(err_string)?
        .receive(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
        .map_err(err_string)?;
    let commit_id = resolve_fetched(&repo, &identifier)?;
    let tree = tree_at_subpath(&repo, commit_id, subpath)?;
    Ok(FetchedTree {
        commit_id: commit_id.to_hex().to_string(),
        entries: read_tree(&tree)?,
    })
}

/// Read `subpath` from a commit already stored in `repo`.
pub fn read_commit_entries(
    repo: &gix::Repository,
    commit_id: gix::ObjectId,
    subpath: &str,
) -> Result<Vec<GitEntry>, String> {
    let tree = tree_at_subpath(repo, commit_id, subpath)?;
    read_tree(&tree)
}

#[cfg(test)]
fn signature() -> gix::actor::Signature {
    gix::actor::Signature {
        name: "Lariv".into(),
        email: "lariv@example.com".into(),
        time: gix::date::Time::new(0, 0),
    }
}

#[cfg(test)]
fn sample_repo(dir: &std::path::Path) -> gix::ObjectId {
    let repo = gix::init(dir).expect("init");
    let readme = repo.write_blob(b"readme").expect("blob").detach();
    let nested = repo.write_blob(b"nested").expect("blob").detach();
    let mut child = gix::objs::Tree::empty();
    child.entries.push(gix::objs::tree::Entry {
        mode: gix::objs::tree::EntryKind::Blob.into(),
        filename: "note.txt".into(),
        oid: nested,
    });
    child.entries.sort();
    let child_id = repo.write_object(&child).expect("tree").detach();
    let mut root = gix::objs::Tree::empty();
    root.entries.push(gix::objs::tree::Entry {
        mode: gix::objs::tree::EntryKind::Blob.into(),
        filename: "README.md".into(),
        oid: readme,
    });
    root.entries.push(gix::objs::tree::Entry {
        mode: gix::objs::tree::EntryKind::Tree.into(),
        filename: "src".into(),
        oid: child_id,
    });
    root.entries.sort();
    let root_id = repo.write_object(&root).expect("root").detach();
    let author = signature();
    let commit_id = repo
        .write_object(&gix::objs::Commit {
            tree: root_id,
            parents: Default::default(),
            author: author.clone(),
            committer: author,
            encoding: None,
            message: "init".into(),
            extra_headers: Vec::new(),
        })
        .expect("commit")
        .detach();
    repo.reference(
        "refs/heads/main",
        commit_id,
        gix::refs::transaction::PreviousValue::Any,
        "init",
    )
    .expect("ref");
    let tag = gix::objs::Tag {
        target: commit_id,
        target_kind: gix::objs::Kind::Commit,
        name: "v1".into(),
        tagger: Some(signature()),
        message: "v1".into(),
        signature: None,
    };
    let tag_id = repo.write_object(&tag).expect("tag").detach();
    repo.reference(
        "refs/tags/v1",
        tag_id,
        gix::refs::transaction::PreviousValue::Any,
        "tag",
    )
    .expect("tag ref");
    commit_id
}

#[cfg(test)]
mod tests {
    use super::{GitEntryKind, fetch_entries, sample_repo};

    #[test]
    fn fetches_subpath_from_a_local_repository() {
        let dir = tempfile::tempdir().expect("temp");
        let commit = sample_repo(dir.path());
        let url = dir.path().to_string_lossy().into_owned();
        let fetched = fetch_entries(&url, &commit.to_hex().to_string(), "src").expect("fetch");
        assert_eq!(fetched.commit_id, commit.to_hex().to_string());
        assert_eq!(fetched.entries.len(), 1);
        let entry = fetched.entries.first().expect("note.txt");
        assert_eq!(entry.name, "note.txt");
        assert!(matches!(&entry.kind, GitEntryKind::File(bytes) if bytes == b"nested"));
    }

    #[test]
    fn fetches_a_branch_name_and_a_tag() {
        let dir = tempfile::tempdir().expect("temp");
        let commit = sample_repo(dir.path());
        let url = dir.path().to_string_lossy().into_owned();
        let expected = commit.to_hex().to_string();
        for identifier in ["main", "refs/heads/main", "v1", "refs/tags/v1"] {
            let fetched = fetch_entries(&url, identifier, "src").expect(identifier);
            assert_eq!(fetched.commit_id, expected);
            let entry = fetched.entries.first().expect("note.txt");
            assert!(matches!(&entry.kind, GitEntryKind::File(bytes) if bytes == b"nested"));
        }
    }
}
