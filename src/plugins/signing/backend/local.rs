//! Local disk key store. For debugging only: private keys are files on disk.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use p256::ecdsa::SigningKey;
use p256::ecdsa::signature::hazmat::PrehashSigner;
use p256::pkcs8::DecodePrivateKey;
use rcgen::{
    CertificateParams, CustomExtension, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose, PKCS_ECDSA_P256_SHA256,
};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use super::super::cert::{self, ADOBE_PDF_SIGNING_EKU, DOCUMENT_SIGNING_EKU, SignerIdentity};
use super::{KeyBackend, KeyError};

pub struct LocalKeys {
    base_dir: PathBuf,
}

impl LocalKeys {
    pub fn new(configured_dir: impl Into<String>) -> Self {
        let configured = configured_dir.into();
        let dir = if configured.trim().is_empty() {
            "signing-keys".to_string()
        } else {
            configured
        };
        let path = PathBuf::from(&dir);
        let base_dir = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        };
        Self { base_dir }
    }

    fn filename_for(user_id: i64) -> String {
        format!("user-{user_id}.pem")
    }

    fn key_path(&self, key_ref: &str) -> Result<PathBuf, KeyError> {
        if !valid_key_ref(key_ref) {
            return Err(KeyError::msg("invalid key reference"));
        }
        Ok(self.base_dir.join(key_ref))
    }

    async fn ensure_dir(&self) -> Result<&Path, KeyError> {
        tokio::fs::create_dir_all(&self.base_dir).await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tokio::fs::set_permissions(&self.base_dir, std::fs::Permissions::from_mode(0o700))
                .await?;
        }
        Ok(&self.base_dir)
    }

    async fn read_pem(&self, key_ref: &str) -> Result<String, KeyError> {
        let path = self.key_path(key_ref)?;
        tokio::fs::read_to_string(&path)
            .await
            .map_err(|err| KeyError::msg(format!("read signing key: {err}")))
    }
}

fn valid_key_ref(key_ref: &str) -> bool {
    !key_ref.is_empty()
        && !key_ref.contains('/')
        && !key_ref.contains('\\')
        && !key_ref.contains("..")
        && key_ref
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
}

fn load_signing_key(pem: &str) -> Result<SigningKey, KeyError> {
    SigningKey::from_pkcs8_pem(pem)
        .map_err(|err| KeyError::msg(format!("parse signing key: {err}")))
}

fn self_signed_cert(pem: &str, identity: &SignerIdentity) -> Result<Vec<u8>, KeyError> {
    let key = KeyPair::from_pkcs8_pem_and_sign_algo(pem, &PKCS_ECDSA_P256_SHA256)
        .map_err(|err| KeyError::msg(format!("load signing key: {err}")))?;
    let mut name = DistinguishedName::new();
    name.push(DnType::CommonName, &identity.authority_name);
    let mut params = CertificateParams::default();
    params.distinguished_name = name;
    params.is_ca = IsCa::NoCa;
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::ContentCommitment,
    ];
    params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::Other(DOCUMENT_SIGNING_EKU.to_vec()),
        ExtendedKeyUsagePurpose::Other(ADOBE_PDF_SIGNING_EKU.to_vec()),
    ];
    let now = time::OffsetDateTime::now_utc();
    let seconds = i64::try_from(identity.validity.as_secs())
        .map_err(|_| KeyError::msg("validity duration is too large"))?;
    params.not_before = now;
    params.not_after = now
        .checked_add(time::Duration::seconds(seconds))
        .ok_or_else(|| KeyError::msg("validity duration is too large"))?;
    let mut basic = CustomExtension::from_oid_content(
        cert::basic_constraints_oid(),
        cert::basic_constraints_value()?,
    );
    basic.set_criticality(false);
    let subject_key = CustomExtension::from_oid_content(
        cert::subject_key_identifier_oid(),
        cert::subject_key_identifier_value(key.public_key_raw())?,
    );
    params.custom_extensions = vec![basic, subject_key];
    let signed = params
        .self_signed(&key)
        .map_err(|err| KeyError::msg(format!("build signer certificate: {err}")))?;
    Ok(signed.der().as_ref().to_vec())
}

#[async_trait]
impl KeyBackend for LocalKeys {
    async fn create_key(&self, user_id: i64) -> Result<String, KeyError> {
        self.ensure_dir().await?;
        let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
            .map_err(|err| KeyError::msg(format!("generate signing key: {err}")))?;
        let pem = key.serialize_pem();
        let filename = Self::filename_for(user_id);
        let path = self.key_path(&filename)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            options.mode(0o600);
        }
        let mut file = match options.open(&path).await {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => return Ok(filename),
            Err(err) => {
                return Err(KeyError::msg(format!("create signing key file: {err}")));
            }
        };
        file.write_all(pem.as_bytes())
            .await
            .map_err(|err| KeyError::msg(format!("write signing key: {err}")))?;
        file.flush().await?;
        Ok(filename)
    }

    async fn sign_digest(&self, key_ref: &str, digest: &[u8]) -> Result<Vec<u8>, KeyError> {
        let pem = self.read_pem(key_ref).await?;
        let key = load_signing_key(&pem)?;
        let signature: p256::ecdsa::DerSignature = key
            .sign_prehash(digest)
            .map_err(|err| KeyError::msg(format!("sign digest: {err}")))?;
        Ok(signature.as_bytes().to_vec())
    }

    async fn certificate_der(
        &self,
        key_ref: &str,
        identity: &SignerIdentity,
    ) -> Result<Vec<u8>, KeyError> {
        let pem = self.read_pem(key_ref).await?;
        self_signed_cert(&pem, identity)
    }

    fn debug_local(&self) -> bool {
        true
    }
}
