//! Skill zip import/export — Go-compatible `index.json` + flat file layout.

use std::collections::HashMap;
use std::io::{Cursor, Write};

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::plugins::{
    filesystem::{
        entities::filesystem_node::Entity as VNodeEntity,
        node::{self, NodeFile},
        storage::DynFilestore,
        zip::read_file_bytes,
    },
    llm_assistant::{
        entities::skill::{self, Entity as SkillEntity},
        handlers::skills::{load_files_for_skill, sync_skill_files},
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillExportJson {
    pub name: String,
    pub description: String,
    pub content: String,
    pub files: Vec<String>,
}

/// .
pub fn sanitize_filename(s: &str) -> String {
    let out: String = s
        .chars()
        .map(|r| {
            if r.is_ascii_alphanumeric() || r == '-' || r == '_' {
                r
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() {
        "skill".into()
    } else {
        out
    }
}

fn read_zip_map(zip_bytes: &[u8]) -> Result<HashMap<String, Vec<u8>>, String> {
    let reader = Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(reader).map_err(|_| "failed to parse zip file".to_string())?;
    let mut map = HashMap::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        if file.is_dir() {
            continue;
        }
        let name = file.name().to_string();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut data).map_err(|e| e.to_string())?;
        map.insert(name, data);
    }
    Ok(map)
}

pub async fn export_skill(
    db: &DatabaseConnection,
    store: &DynFilestore,
    skill_id: i64,
) -> Result<(Vec<u8>, String), String> {
    let skill = SkillEntity::find_by_id(skill_id)
        .filter(skill::Column::DeletedAt.is_null())
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "skill not found".to_string())?;

    let files = load_files_for_skill(db, skill_id).await;
    let export = SkillExportJson {
        name: skill.name.clone(),
        description: skill.description,
        content: skill.content,
        files: files.iter().map(|(_, name)| name.clone()).collect(),
    };

    let mut buf = Vec::new();
    {
        let mut writer = ZipWriter::new(Cursor::new(&mut buf));
        let options = SimpleFileOptions::default();

        let index_bytes = serde_json::to_string_pretty(&export).map_err(|e| e.to_string())?;
        writer
            .start_file("index.json", options)
            .map_err(|e| e.to_string())?;
        writer
            .write_all(index_bytes.as_bytes())
            .map_err(|e| e.to_string())?;

        for (vnode_id, name) in &files {
            let vnode = VNodeEntity::find_by_id(*vnode_id)
                .one(db)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("file node {vnode_id} not found"))?;
            let bytes = read_file_bytes(store, &vnode)
                .await
                .map_err(|e| e.to_string())?;
            writer.start_file(name, options).map_err(|e| e.to_string())?;
            writer.write_all(&bytes).map_err(|e| e.to_string())?;
        }

        writer.finish().map_err(|e| e.to_string())?;
    }

    Ok((buf, sanitize_filename(&export.name)))
}

async fn cleanup_import(
    db: &DatabaseConnection,
    store: &DynFilestore,
    node_ids: &[i64],
    stored_paths: &[String],
) {
    for id in node_ids {
        let _ = VNodeEntity::delete_by_id(*id).exec(db).await;
    }
    for path in stored_paths {
        let _ = store.delete(path).await;
    }
}

pub async fn import_skill(
    db: &DatabaseConnection,
    store: &DynFilestore,
    zip_bytes: &[u8],
) -> Result<skill::Model, String> {
    let entries = read_zip_map(zip_bytes)?;
    let index_bytes = entries
        .get("index.json")
        .ok_or_else(|| "index.json is missing in the zip archive".to_string())?;
    let export: SkillExportJson =
        serde_json::from_slice(index_bytes).map_err(|e| e.to_string())?;

    if export.name.trim().is_empty() {
        return Err("skill name is required in index.json".to_string());
    }

    let mut created_nodes: Vec<i64> = Vec::new();
    let mut stored_paths: Vec<String> = Vec::new();

    for filename in &export.files {
        let data = entries.get(filename).ok_or_else(|| {
            format!("file {filename:?} specified in index.json is missing from the zip archive")
        })?;

        let vnode = match node::create(
            db,
            store,
            filename.clone(),
            false,
            Some(NodeFile::Bytes {
                filename: filename.clone(),
                data: data.clone(),
            }),
            None,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                cleanup_import(db, store, &created_nodes, &stored_paths).await;
                return Err(format!("failed to create file node {filename:?}: {e}"));
            }
        };
        if let Some(ref path) = vnode.file_path {
            stored_paths.push(path.clone());
        }
        created_nodes.push(vnode.id);
    }

    let now = Utc::now();
    let skill_model = match (skill::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        deleted_at: Set(None),
        name: Set(export.name),
        description: Set(export.description),
        content: Set(export.content),
    })
    .insert(db)
    .await
    {
        Ok(s) => s,
        Err(e) => {
            cleanup_import(db, store, &created_nodes, &stored_paths).await;
            return Err(format!("failed to save skill: {e}"));
        }
    };

    if let Err(e) = sync_skill_files(db, skill_model.id, &created_nodes).await {
        let _ = SkillEntity::delete_by_id(skill_model.id).exec(db).await;
        cleanup_import(db, store, &created_nodes, &stored_paths).await;
        return Err(format!("failed to link skill files: {e}"));
    }

    Ok(skill_model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, Database, Schema};
    use std::io::Write;

    use crate::plugins::filesystem::{
        entities::filesystem_node,
        node::{self, NodeFile},
        storage::LocalFilestore,
    };
    use crate::plugins::llm_assistant::{
        entities::{skill, skill_file_link},
        handlers::skills::{load_files_for_skill, sync_skill_files},
    };

    #[test]
    fn sanitize_filename_replaces_invalid_chars() {
        assert_eq!(sanitize_filename("My Skill v2!"), "My_Skill_v2_");
        assert_eq!(sanitize_filename("..."), "___");
        assert_eq!(sanitize_filename(""), "skill");
    }

    #[test]
    fn export_json_roundtrip() {
        let j = SkillExportJson {
            name: "test".into(),
            description: "d".into(),
            content: "c".into(),
            files: vec!["a.py".into()],
        };
        let s = serde_json::to_string(&j).unwrap();
        let back: SkillExportJson = serde_json::from_str(&s).unwrap();
        assert_eq!(j, back);
    }

    async fn setup_zip_db() -> (sea_orm::DatabaseConnection, LocalFilestore) {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("sqlite memory");
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        db.execute(backend.build(&schema.create_table_from_entity(skill::Entity)))
            .await
            .expect("skills");
        db.execute(backend.build(&schema.create_table_from_entity(filesystem_node::Entity)))
            .await
            .expect("vnodes");
        db.execute(
            backend.build(&schema.create_table_from_entity(skill_file_link::Entity)),
        )
        .await
        .expect("create link table");
        let dir = std::env::temp_dir().join(format!("lariv-skill-zip-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        (db, LocalFilestore::new(dir.to_string_lossy()))
    }

    fn build_test_zip() -> Vec<u8> {
        let export = SkillExportJson {
            name: "zip-skill".into(),
            description: "desc".into(),
            content: "body".into(),
            files: vec!["helper.py".into()],
        };
        let mut buf = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options = zip::write::SimpleFileOptions::default();
            let index = serde_json::to_string_pretty(&export).unwrap();
            writer.start_file("index.json", options).unwrap();
            writer.write_all(index.as_bytes()).unwrap();
            writer.start_file("helper.py", options).unwrap();
            writer.write_all(b"print('hi')").unwrap();
            writer.finish().unwrap();
        }
        buf
    }

    #[tokio::test]
    async fn import_export_round_trip() {
        let (db, store) = setup_zip_db().await;
        let store_ref: &dyn crate::plugins::filesystem::storage::Filestore = &store;

        let imported = import_skill(&db, store_ref, &build_test_zip())
            .await
            .expect("import");
        assert_eq!(imported.name, "zip-skill");
        assert_eq!(imported.content, "body");

        let files = load_files_for_skill(&db, imported.id).await;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].1, "helper.py");

        let (zip_bytes, filename) = export_skill(&db, store_ref, imported.id)
            .await
            .expect("export");
        assert_eq!(filename, "zip-skill");

        let entries = read_zip_map(&zip_bytes).expect("parse export zip");
        let index: SkillExportJson =
            serde_json::from_slice(entries.get("index.json").unwrap()).unwrap();
        assert_eq!(index.name, "zip-skill");
        assert_eq!(index.files, vec!["helper.py".to_string()]);
        assert_eq!(
            entries.get("helper.py").map(|b| b.as_slice()),
            Some(b"print('hi')" as &[u8]),
        );
    }

    #[tokio::test]
    async fn export_existing_skill_with_file() {
        let (db, store) = setup_zip_db().await;
        let store_ref: &dyn crate::plugins::filesystem::storage::Filestore = &store;

        let vnode = node::create(
            &db,
            store_ref,
            "tool.rs".into(),
            false,
            Some(NodeFile::Bytes {
                filename: "tool.rs".into(),
                data: b"fn main() {}".to_vec(),
            }),
            None,
        )
        .await
        .expect("vnode");

        let now = Utc::now();
        let skill_model = (skill::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            deleted_at: Set(None),
            name: Set("exported".into()),
            description: Set(String::new()),
            content: Set("do thing".into()),
        })
        .insert(&db)
        .await
        .expect("skill");
        sync_skill_files(&db, skill_model.id, &[vnode.id])
            .await
            .expect("link");

        let (bytes, name) = export_skill(&db, store_ref, skill_model.id)
            .await
            .expect("export");
        assert_eq!(name, "exported");
        let map = read_zip_map(&bytes).unwrap();
        assert!(map.contains_key("index.json"));
        assert!(map.contains_key("tool.rs"));
    }
}
