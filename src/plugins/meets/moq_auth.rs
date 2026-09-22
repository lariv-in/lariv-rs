//! MoQ JWT minting for joined participants (`moq-token` + `moq-room` claims).

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use moq_token::{Algorithm, Key, KeyId};
use tempfile::TempDir;

use super::config::MeetsConfig;

pub const JWT_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Signing key and on-disk directory for the embedded relay's JWT verifier.
pub struct MoqAuthState {
    key: Key,
    key_dir: TempDir,
}

impl MoqAuthState {
    pub fn load(config: &MeetsConfig) -> anyhow::Result<Self> {
        let key = if config.transport.auth_key_file.is_empty() {
            Key::generate(Algorithm::HS256, Some(KeyId::random()))?
        } else {
            Key::from_file(&config.transport.auth_key_file)?
        };
        let key_dir = TempDir::new()?;
        let kid = key
            .kid
            .as_ref()
            .map(|id| id.as_ref())
            .unwrap_or("lariv-meets");
        let path = key_dir.path().join(format!("{kid}.jwk"));
        key.to_file(&path)?;
        Ok(Self { key, key_dir })
    }

    pub fn key_dir(&self) -> PathBuf {
        self.key_dir.path().to_path_buf()
    }

    pub fn token_for_participant(
        &self,
        room_code: &str,
        joined_user_id: i64,
    ) -> anyhow::Result<String> {
        let identity = joined_user_id.to_string();
        let mut claims = moq_room::claims(format!("meets/{room_code}"), &identity)
            .map_err(anyhow::Error::msg)?;
        claims = claims
            .with_expires(SystemTime::now() + JWT_TTL)
            .with_issued(SystemTime::now());
        claims.validate()?;
        self.key.sign(&claims).map_err(anyhow::Error::msg)
    }
}
