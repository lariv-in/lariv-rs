//! Signer certificate profile shared by the local and Cloud KMS backends.
//!
//! The extensions match an end-entity document-signing certificate:
//! basic constraints `CA:FALSE`, a subject key identifier hashed from the public
//! key, critical key usage `digitalSignature` and `nonRepudiation`, and extended
//! key usage for document signing plus Adobe PDF signing.

use const_oid::{AssociatedOid, ObjectIdentifier};
use der::Encode;
use der::asn1::OctetString;
use sha1::{Digest, Sha1};
use x509_cert::ext::Extension;
use x509_cert::ext::pkix::constraints::BasicConstraints;
use x509_cert::ext::pkix::{ExtendedKeyUsage, KeyUsage, KeyUsages, SubjectKeyIdentifier};
use x509_cert::name::Name;

use super::backend::KeyError;

/// `id-kp-documentSigning` (1.3.6.1.5.5.7.3.36).
pub const DOCUMENT_SIGNING_EKU: &[u64] = &[1, 3, 6, 1, 5, 5, 7, 3, 36];

/// Adobe PDF signing (1.2.840.113583.1.1.5).
pub const ADOBE_PDF_SIGNING_EKU: &[u64] = &[1, 2, 840, 113583, 1, 1, 5];

const BASIC_CONSTRAINTS_OID: &[u64] = &[2, 5, 29, 19];
const SUBJECT_KEY_IDENTIFIER_OID: &[u64] = &[2, 5, 29, 14];

/// Name and lifetime written into the signer certificate at sign time.
#[derive(Debug, Clone)]
pub struct SignerIdentity {
    pub authority_name: String,
    pub validity: std::time::Duration,
}

impl SignerIdentity {
    pub fn from_preferences(name: &str, validity_ns: i64) -> Result<Self, String> {
        let authority_name =
            crate::plugins::documents::preferences::validate_authority_name(name)?.to_string();
        let nanos =
            u64::try_from(validity_ns).map_err(|_| "Validity duration is too large".to_string())?;
        if nanos < 1_000_000_000 {
            return Err("Validity duration must be at least 1 second".to_string());
        }
        Ok(Self {
            authority_name,
            validity: std::time::Duration::from_nanos(nanos),
        })
    }
}

/// SHA-1 of the raw subject public key, the OpenSSL `subjectKeyIdentifier = hash` value.
pub fn subject_key_id(public_key: &[u8]) -> Vec<u8> {
    Sha1::digest(public_key).to_vec()
}

/// Inner DER for a non-critical `CA:FALSE` basic constraints extension.
pub fn basic_constraints_value() -> Result<Vec<u8>, KeyError> {
    BasicConstraints {
        ca: false,
        path_len_constraint: None,
    }
    .to_der()
    .map_err(|err| KeyError::msg(format!("basic constraints: {err}")))
}

/// Inner DER for `subjectKeyIdentifier = hash`.
pub fn subject_key_identifier_value(public_key: &[u8]) -> Result<Vec<u8>, KeyError> {
    let id = OctetString::new(subject_key_id(public_key))
        .map_err(|err| KeyError::msg(format!("subject key identifier: {err}")))?;
    SubjectKeyIdentifier(id)
        .to_der()
        .map_err(|err| KeyError::msg(format!("subject key identifier: {err}")))
}

fn key_usage() -> KeyUsage {
    KeyUsage(KeyUsages::DigitalSignature | KeyUsages::NonRepudiation)
}

fn extended_key_usage() -> Result<ExtendedKeyUsage, KeyError> {
    let document = ObjectIdentifier::new("1.3.6.1.5.5.7.3.36")
        .map_err(|err| KeyError::msg(format!("document signing usage: {err}")))?;
    let adobe = ObjectIdentifier::new("1.2.840.113583.1.1.5")
        .map_err(|err| KeyError::msg(format!("adobe signing usage: {err}")))?;
    Ok(ExtendedKeyUsage(vec![document, adobe]))
}

fn extension(
    oid: ObjectIdentifier,
    critical: bool,
    value: impl Encode,
) -> Result<Extension, KeyError> {
    let der = value
        .to_der()
        .map_err(|err| KeyError::msg(format!("certificate extension: {err}")))?;
    let extn_value = OctetString::new(der)
        .map_err(|err| KeyError::msg(format!("certificate extension: {err}")))?;
    Ok(Extension {
        extn_id: oid,
        critical,
        extn_value,
    })
}

/// End-entity extensions for a PDF signer certificate.
pub fn signing_extensions(public_key: &[u8]) -> Result<Vec<Extension>, KeyError> {
    let ski = OctetString::new(subject_key_id(public_key))
        .map_err(|err| KeyError::msg(format!("subject key identifier: {err}")))?;
    Ok(vec![
        extension(
            BasicConstraints::OID,
            false,
            BasicConstraints {
                ca: false,
                path_len_constraint: None,
            },
        )?,
        extension(SubjectKeyIdentifier::OID, false, SubjectKeyIdentifier(ski))?,
        extension(KeyUsage::OID, true, key_usage())?,
        extension(ExtendedKeyUsage::OID, false, extended_key_usage()?)?,
    ])
}

/// `CN=` distinguished name. Commas and other RFC 4514 characters are escaped.
pub fn common_name(authority: &str) -> Result<Name, KeyError> {
    let dn = format!("CN={}", ldap_escape(authority));
    dn.parse()
        .map_err(|err| KeyError::msg(format!("certificate name: {err}")))
}

/// rcgen OID arcs for the two extensions it does not emit when `IsCa::NoCa`.
pub fn basic_constraints_oid() -> &'static [u64] {
    BASIC_CONSTRAINTS_OID
}

pub fn subject_key_identifier_oid() -> &'static [u64] {
    SUBJECT_KEY_IDENTIFIER_OID
}

fn ldap_escape(value: &str) -> String {
    let mut out = String::new();
    let last = value.chars().count().saturating_sub(1);
    for (index, ch) in value.chars().enumerate() {
        let special = matches!(ch, '"' | '+' | ',' | ';' | '<' | '>' | '\\')
            || (ch == '#' && index == 0)
            || (ch == ' ' && (index == 0 || index == last));
        if special {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}
