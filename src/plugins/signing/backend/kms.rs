//! Google Cloud KMS backend. The private key stays in KMS.

use async_trait::async_trait;
use const_oid::db::rfc5912::ECDSA_WITH_SHA_256;
use der::DecodePem;
use der::Encode;
use der::asn1::BitString;
use google_cloud_kms_v1::client::KeyManagementService;
use google_cloud_kms_v1::model::crypto_key::CryptoKeyPurpose;
use google_cloud_kms_v1::model::crypto_key_version::CryptoKeyVersionAlgorithm;
use google_cloud_kms_v1::model::digest::Digest as DigestKind;
use google_cloud_kms_v1::model::{CryptoKey, CryptoKeyVersionTemplate, Digest, ProtectionLevel};
use sha2::{Digest as Sha256Digest, Sha256};
use spki::{AlgorithmIdentifierOwned, SubjectPublicKeyInfoOwned};
use tokio::sync::OnceCell;
use x509_cert::certificate::{Certificate, TbsCertificate, Version};
use x509_cert::serial_number::SerialNumber;
use x509_cert::time::Validity;

use super::super::cert::{self, SignerIdentity};
use super::super::config::SigningConfig;
use super::{KeyBackend, KeyError};

/// Operations the Cloud KMS client must provide. Tests substitute a fake.
#[async_trait]
pub trait KmsClient: Send + Sync {
    async fn create_signing_key(&self, user_id: i64) -> Result<String, KeyError>;
    async fn sign_sha256(&self, key_ref: &str, digest: &[u8]) -> Result<Vec<u8>, KeyError>;
    async fn public_key_pem(&self, key_ref: &str) -> Result<String, KeyError>;
}

pub struct KmsKeys<C: KmsClient = GoogleKms> {
    client: C,
}

impl KmsKeys<GoogleKms> {
    pub fn new(config: SigningConfig) -> Self {
        Self {
            client: GoogleKms::new(config),
        }
    }
}

impl<C: KmsClient> KmsKeys<C> {
    pub fn from_client(client: C) -> Self {
        Self { client }
    }
}

#[async_trait]
impl<C: KmsClient> KeyBackend for KmsKeys<C> {
    async fn create_key(&self, user_id: i64) -> Result<String, KeyError> {
        self.client.create_signing_key(user_id).await
    }

    async fn sign_digest(&self, key_ref: &str, digest: &[u8]) -> Result<Vec<u8>, KeyError> {
        self.client.sign_sha256(key_ref, digest).await
    }

    async fn certificate_der(
        &self,
        key_ref: &str,
        identity: &SignerIdentity,
    ) -> Result<Vec<u8>, KeyError> {
        let pem = self.client.public_key_pem(key_ref).await?;
        let (tbs, signature_alg, digest) = unsigned_certificate(&pem, identity)?;
        let signature = self.client.sign_sha256(key_ref, &digest).await?;
        assemble_certificate(tbs, signature_alg, &signature)
    }
}

pub struct GoogleKms {
    config: SigningConfig,
    client: OnceCell<KeyManagementService>,
}

impl GoogleKms {
    pub fn new(config: SigningConfig) -> Self {
        Self {
            config,
            client: OnceCell::new(),
        }
    }

    async fn client(&self) -> Result<&KeyManagementService, KeyError> {
        self.client.get_or_try_init(|| connect(&self.config)).await
    }

    fn key_ring_parent(&self) -> String {
        format!(
            "projects/{}/locations/{}/keyRings/{}",
            self.config.kms_project.trim(),
            self.config.kms_location.trim(),
            self.config.kms_key_ring.trim()
        )
    }
}

async fn connect(config: &SigningConfig) -> Result<KeyManagementService, KeyError> {
    let mut builder = KeyManagementService::builder();
    let credentials_file = config.kms_credentials_file.trim();
    if !credentials_file.is_empty() {
        let json = tokio::fs::read_to_string(credentials_file)
            .await
            .map_err(|err| KeyError::msg(format!("read kms credentials: {err}")))?;
        let value: serde_json::Value = serde_json::from_str(&json)
            .map_err(|err| KeyError::msg(format!("parse kms credentials: {err}")))?;
        let credentials = google_cloud_auth::credentials::service_account::Builder::new(value)
            .build()
            .map_err(|err| KeyError::msg(format!("kms credentials: {err}")))?;
        builder = builder.with_credentials(credentials);
    }
    builder
        .build()
        .await
        .map_err(|err| KeyError::msg(format!("kms client: {err}")))
}

#[async_trait]
impl KmsClient for GoogleKms {
    async fn create_signing_key(&self, user_id: i64) -> Result<String, KeyError> {
        let client = self.client().await?;
        let parent = self.key_ring_parent();
        let crypto_key_id = format!("user-{user_id}");
        let template = CryptoKeyVersionTemplate::new()
            .set_algorithm(CryptoKeyVersionAlgorithm::EcSignP256Sha256)
            .set_protection_level(ProtectionLevel::Software);
        let crypto_key = CryptoKey::new()
            .set_purpose(CryptoKeyPurpose::AsymmetricSign)
            .set_version_template(template);
        let version = format!("{parent}/cryptoKeys/{crypto_key_id}/cryptoKeyVersions/1");
        match client
            .create_crypto_key()
            .set_parent(&parent)
            .set_crypto_key_id(&crypto_key_id)
            .set_crypto_key(crypto_key)
            .send()
            .await
        {
            Ok(created) => {
                if created.name.is_empty() {
                    Ok(version)
                } else {
                    Ok(format!("{}/cryptoKeyVersions/1", created.name))
                }
            }
            Err(err) if already_exists(&err) => Ok(version),
            Err(err) => Err(KeyError::msg(format!("create kms key: {err}"))),
        }
    }

    async fn sign_sha256(&self, key_ref: &str, digest: &[u8]) -> Result<Vec<u8>, KeyError> {
        let client = self.client().await?;
        let digest_msg = Digest::new().set_digest(Some(DigestKind::Sha256(
            bytes::Bytes::copy_from_slice(digest),
        )));
        let response = client
            .asymmetric_sign()
            .set_name(key_ref)
            .set_digest(digest_msg)
            .send()
            .await
            .map_err(|err| KeyError::msg(format!("kms sign: {err}")))?;
        Ok(response.signature.to_vec())
    }

    async fn public_key_pem(&self, key_ref: &str) -> Result<String, KeyError> {
        let client = self.client().await?;
        let response = client
            .get_public_key()
            .set_name(key_ref)
            .send()
            .await
            .map_err(|err| KeyError::msg(format!("kms public key: {err}")))?;
        if response.pem.trim().is_empty() {
            return Err(KeyError::msg("kms public key was empty"));
        }
        Ok(response.pem)
    }
}

fn already_exists(err: &impl std::fmt::Display) -> bool {
    let text = err.to_string();
    text.contains("ALREADY_EXISTS") || text.contains("already exists")
}

fn unsigned_certificate(
    pem: &str,
    identity: &SignerIdentity,
) -> Result<(TbsCertificate, AlgorithmIdentifierOwned, Vec<u8>), KeyError> {
    let spki = SubjectPublicKeyInfoOwned::from_pem(pem)
        .map_err(|err| KeyError::msg(format!("parse public key: {err}")))?;
    let public_key = spki
        .subject_public_key
        .as_bytes()
        .ok_or_else(|| KeyError::msg("public key bit string is not byte aligned"))?;
    let extensions = cert::signing_extensions(public_key)?;
    let name = cert::common_name(&identity.authority_name)?;
    let validity = Validity::from_now(identity.validity)
        .map_err(|err| KeyError::msg(format!("certificate validity: {err}")))?;
    let serial = SerialNumber::new(&[1, 2, 3, 4])
        .map_err(|err| KeyError::msg(format!("certificate serial: {err}")))?;
    let signature_alg = ecdsa_sha256_alg()?;
    let tbs = TbsCertificate {
        version: Version::V3,
        serial_number: serial,
        signature: signature_alg.clone(),
        issuer: name.clone(),
        validity,
        subject: name,
        subject_public_key_info: spki,
        issuer_unique_id: None,
        subject_unique_id: None,
        extensions: Some(extensions),
    };
    let tbs_der = tbs
        .to_der()
        .map_err(|err| KeyError::msg(format!("encode certificate: {err}")))?;
    let digest = Sha256::digest(&tbs_der).to_vec();
    Ok((tbs, signature_alg, digest))
}

fn assemble_certificate(
    tbs: TbsCertificate,
    signature_alg: AlgorithmIdentifierOwned,
    signature: &[u8],
) -> Result<Vec<u8>, KeyError> {
    let signature_bits = BitString::from_bytes(signature)
        .map_err(|err| KeyError::msg(format!("certificate signature: {err}")))?;
    let cert = Certificate {
        tbs_certificate: tbs,
        signature_algorithm: signature_alg,
        signature: signature_bits,
    };
    cert.to_der()
        .map_err(|err| KeyError::msg(format!("encode certificate: {err}")))
}

fn ecdsa_sha256_alg() -> Result<AlgorithmIdentifierOwned, KeyError> {
    Ok(AlgorithmIdentifierOwned {
        oid: ECDSA_WITH_SHA_256,
        parameters: None,
    })
}
