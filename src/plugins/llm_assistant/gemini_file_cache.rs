//! Gemini Files API URI cache keyed by VNode id + content hash.
//!
//! Files API objects expire (typically ~48h). Reuse a stored URI when the VNode
//! bytes have not changed, the row is not past `expires_at`, and `get_file`
//! still reports `ACTIVE`.

use chrono::{DateTime, Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use sha2::{Digest, Sha256};

use crate::genai::GenaiClient;

use super::entities::gemini_file_cache::{self, Entity as CacheEntity};

const LOG_TARGET: &str = "llm_assistant::gemini_file_cache";
/// Treat a cache row as expired this far before `expires_at` to avoid races.
const EXPIRY_SKEW: Duration = Duration::minutes(2);

#[derive(Debug, Clone)]
pub struct CachedGeminiFile {
    pub file_uri: String,
    pub mime_type: String,
    pub display_name: String,
    pub file_name: String,
}

pub fn content_sha256(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

/// True when the row matches `hash` and is not past its stored expiry (with skew).
pub fn row_is_fresh(row: &gemini_file_cache::Model, hash: &[u8], now: DateTime<Utc>) -> bool {
    row.content_sha256 == hash && !expires_soon(row.expires_at, now)
}

fn expires_soon(expires_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    match expires_at {
        None => false,
        Some(at) => at <= now + EXPIRY_SKEW,
    }
}

/// Return a live Files API URI for this VNode, uploading only when the cache misses.
pub async fn resolve_or_upload(
    db: &DatabaseConnection,
    genai: &GenaiClient,
    vnode_id: i64,
    display_name: &str,
    mime_type: &str,
    bytes: &[u8],
) -> Result<(CachedGeminiFile, bool), String> {
    let hash = content_sha256(bytes);
    if let Some(cached) = live_cache_hit(db, genai, vnode_id, &hash).await? {
        return Ok((cached, true));
    }

    let uploaded = genai
        .upload_file(display_name, mime_type, bytes)
        .await
        .map_err(|e| format!("Files API upload: {e}"))?;
    let cached = CachedGeminiFile {
        file_uri: uploaded.uri.clone(),
        mime_type: if uploaded.mime_type.is_empty() {
            mime_type.to_string()
        } else {
            uploaded.mime_type.clone()
        },
        display_name: if uploaded.display_name.is_empty() {
            display_name.to_string()
        } else {
            uploaded.display_name.clone()
        },
        file_name: uploaded.name.clone(),
    };
    upsert_cache(db, vnode_id, &hash, &cached, uploaded.expiration_time).await?;
    Ok((cached, false))
}

async fn live_cache_hit(
    db: &DatabaseConnection,
    genai: &GenaiClient,
    vnode_id: i64,
    hash: &[u8],
) -> Result<Option<CachedGeminiFile>, String> {
    let row = CacheEntity::find()
        .filter(gemini_file_cache::Column::VnodeId.eq(vnode_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    let Some(row) = row else {
        return Ok(None);
    };
    if !row_is_fresh(&row, hash, Utc::now()) {
        return Ok(None);
    }
    match genai.get_file(&row.file_name).await {
        Ok(file) if file.state_is_active() => {
            let cached = CachedGeminiFile {
                file_uri: if file.uri.is_empty() {
                    row.file_uri.clone()
                } else {
                    file.uri
                },
                mime_type: if file.mime_type.is_empty() {
                    row.mime_type.clone()
                } else {
                    file.mime_type
                },
                display_name: row.display_name.clone(),
                file_name: row.file_name.clone(),
            };
            if file.expiration_time != row.expires_at {
                let _ = touch_expiry(db, row.id, file.expiration_time).await;
            }
            Ok(Some(cached))
        }
        Ok(file) => {
            tracing::debug!(
                target: LOG_TARGET,
                vnode_id,
                state = %file.state,
                "cached Gemini file is not ACTIVE; will re-upload"
            );
            Ok(None)
        }
        Err(e) => {
            tracing::debug!(
                target: LOG_TARGET,
                vnode_id,
                error = %e,
                "cached Gemini file lookup failed; will re-upload"
            );
            Ok(None)
        }
    }
}

async fn upsert_cache(
    db: &DatabaseConnection,
    vnode_id: i64,
    hash: &[u8],
    cached: &CachedGeminiFile,
    expires_at: Option<DateTime<Utc>>,
) -> Result<(), String> {
    let now = Utc::now();
    let existing = CacheEntity::find()
        .filter(gemini_file_cache::Column::VnodeId.eq(vnode_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(row) = existing {
        let mut am: gemini_file_cache::ActiveModel = row.into();
        am.updated_at = Set(Some(now));
        am.content_sha256 = Set(hash.to_vec());
        am.file_uri = Set(cached.file_uri.clone());
        am.file_name = Set(cached.file_name.clone());
        am.mime_type = Set(cached.mime_type.clone());
        am.display_name = Set(cached.display_name.clone());
        am.expires_at = Set(expires_at);
        am.update(db).await.map_err(|e| e.to_string())?;
    } else {
        gemini_file_cache::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            vnode_id: Set(vnode_id),
            content_sha256: Set(hash.to_vec()),
            file_uri: Set(cached.file_uri.clone()),
            file_name: Set(cached.file_name.clone()),
            mime_type: Set(cached.mime_type.clone()),
            display_name: Set(cached.display_name.clone()),
            expires_at: Set(expires_at),
        }
        .insert(db)
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn touch_expiry(
    db: &DatabaseConnection,
    id: i64,
    expires_at: Option<DateTime<Utc>>,
) -> Result<(), sea_orm::DbErr> {
    let Some(row) = CacheEntity::find_by_id(id).one(db).await? else {
        return Ok(());
    };
    let mut am: gemini_file_cache::ActiveModel = row.into();
    am.updated_at = Set(Some(Utc::now()));
    am.expires_at = Set(expires_at);
    am.update(db).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::llm_assistant::entities::gemini_file_cache::Model;

    fn row(hash: Vec<u8>, expires_at: Option<DateTime<Utc>>) -> Model {
        Model {
            id: 1,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            vnode_id: 3,
            content_sha256: hash,
            file_uri: "https://example/files/1".into(),
            file_name: "files/1".into(),
            mime_type: "application/pdf".into(),
            display_name: "doc.pdf".into(),
            expires_at,
        }
    }

    #[test]
    fn content_sha256_stable() {
        let a = content_sha256(b"hello");
        let b = content_sha256(b"hello");
        let c = content_sha256(b"world");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn fresh_when_hash_matches_and_no_expiry() {
        let hash = content_sha256(b"hello");
        assert!(row_is_fresh(&row(hash.clone(), None), &hash, Utc::now()));
    }

    #[test]
    fn stale_when_hash_differs() {
        let hash = content_sha256(b"hello");
        let other = content_sha256(b"world");
        assert!(!row_is_fresh(&row(hash, None), &other, Utc::now()));
    }

    #[test]
    fn stale_when_expired_or_within_skew() {
        let hash = content_sha256(b"hello");
        let now = Utc::now();
        assert!(!row_is_fresh(
            &row(hash.clone(), Some(now - Duration::minutes(1))),
            &hash,
            now
        ));
        assert!(!row_is_fresh(
            &row(hash.clone(), Some(now + Duration::minutes(1))),
            &hash,
            now
        ));
        assert!(row_is_fresh(
            &row(hash, Some(now + Duration::hours(1))),
            &content_sha256(b"hello"),
            now
        ));
    }
}
