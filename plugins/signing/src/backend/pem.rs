//! In-memory P-256 PKCS#8 PEM signer, and the certificate helpers shared with the local backend.

use async_trait::async_trait;
use p256::ecdsa::SigningKey;
use p256::ecdsa::signature::hazmat::PrehashSigner;
use p256::pkcs8::DecodePrivateKey;
use rcgen::{
    CertificateParams, CustomExtension, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose, PKCS_ECDSA_P256_SHA256,
};

use super::super::cert::{self, ADOBE_PDF_SIGNING_EKU, DOCUMENT_SIGNING_EKU, SignerIdentity};
use super::{KeyBackend, KeyError};

/// Signs with a PKCS#8 PEM held in memory. The key is not written to disk.
pub struct PemKeyBackend {
    pem: String,
}

impl PemKeyBackend {
    /// Reject anything that is not a P-256 PKCS#8 private key.
    pub fn parse(pem: &str) -> Result<Self, KeyError> {
        let pem = pem.trim();
        if pem.is_empty() {
            return Err(KeyError::msg("signing key is empty"));
        }
        load_signing_key(pem)?;
        Ok(Self {
            pem: pem.to_string(),
        })
    }
}

pub(super) fn load_signing_key(pem: &str) -> Result<SigningKey, KeyError> {
    SigningKey::from_pkcs8_pem(pem)
        .map_err(|err| KeyError::msg(format!("parse signing key: {err}")))
}

pub(super) fn sign_pem_digest(pem: &str, digest: &[u8]) -> Result<Vec<u8>, KeyError> {
    let key = load_signing_key(pem)?;
    let signature: p256::ecdsa::DerSignature = key
        .sign_prehash(digest)
        .map_err(|err| KeyError::msg(format!("sign digest: {err}")))?;
    Ok(signature.as_bytes().to_vec())
}

pub(super) fn self_signed_cert(pem: &str, identity: &SignerIdentity) -> Result<Vec<u8>, KeyError> {
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
impl KeyBackend for PemKeyBackend {
    async fn create_key(&self, _user_id: i64) -> Result<String, KeyError> {
        Err(KeyError::msg("in-memory signing key cannot create keys"))
    }

    async fn sign_digest(&self, _key_ref: &str, digest: &[u8]) -> Result<Vec<u8>, KeyError> {
        sign_pem_digest(&self.pem, digest)
    }

    async fn certificate_der(
        &self,
        _key_ref: &str,
        identity: &SignerIdentity,
    ) -> Result<Vec<u8>, KeyError> {
        self_signed_cert(&self.pem, identity)
    }
}
