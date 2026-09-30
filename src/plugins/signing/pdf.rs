//! Detached CMS signature embedded in an incremental PDF update.

use cms::cert::{CertificateChoices, IssuerAndSerialNumber};
use cms::content_info::{CmsVersion, ContentInfo};
use cms::signed_data::{
    CertificateSet, DigestAlgorithmIdentifiers, EncapsulatedContentInfo, SignatureValue,
    SignedData, SignerIdentifier, SignerInfo, SignerInfos,
};
use const_oid::db::rfc5911::{ID_CONTENT_TYPE, ID_DATA, ID_MESSAGE_DIGEST, ID_SIGNED_DATA};
use const_oid::db::rfc5912::{ECDSA_WITH_SHA_256, ID_SHA_256};
use der::asn1::{Null, OctetString, SetOfVec};
use der::{Any, Decode, Encode};
use lopdf::{Dictionary, IncrementalDocument, Object, ObjectId, StringFormat};
use sha2::{Digest, Sha256};
use spki::AlgorithmIdentifierOwned;
use x509_cert::attr::{Attribute, Attributes};
use x509_cert::certificate::Certificate;

use super::backend::{KeyBackend, KeyError};

const CONTENTS_LEN: usize = 8192;
const BYTE_RANGE_PLACEHOLDER: i64 = 1_111_111_111;
const BYTE_RANGE_WIDTH: usize = 10;

#[derive(Debug)]
pub struct PdfSignError(pub String);

impl PdfSignError {
    fn msg(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl std::fmt::Display for PdfSignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PdfSignError {}

impl From<KeyError> for PdfSignError {
    fn from(value: KeyError) -> Self {
        Self(value.0)
    }
}

pub fn is_pdf_name(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".pdf") && name.len() > 4
}

pub fn is_pdf_bytes(bytes: &[u8]) -> bool {
    bytes.starts_with(b"%PDF-")
}

pub fn signed_filename(name: &str) -> String {
    let stem = if is_pdf_name(name) {
        name.get(..name.len().saturating_sub(4)).unwrap_or(name)
    } else {
        name
    };
    let stem = stem.trim();
    if stem.is_empty() {
        "signed.pdf".to_string()
    } else {
        format!("{stem}-signed.pdf")
    }
}

/// Append an incremental update that signs `pdf` with `backend`.
pub async fn sign_pdf(
    pdf: &[u8],
    backend: &dyn KeyBackend,
    key_ref: &str,
    identity: &super::cert::SignerIdentity,
) -> Result<Vec<u8>, PdfSignError> {
    if !is_pdf_bytes(pdf) {
        return Err(PdfSignError::msg("file is not a PDF"));
    }
    let mut prepared = prepare_incremental(pdf)?;
    patch_byte_range(&mut prepared)?;
    let (head, tail) = byte_ranges(&prepared)?;
    let mut hasher = Sha256::new();
    hasher.update(head);
    hasher.update(tail);
    let pdf_digest = hasher.finalize();
    let cms = build_cms(pdf_digest.as_slice(), backend, key_ref, identity).await?;
    embed_cms(&mut prepared, &cms)?;
    Ok(prepared)
}

fn prepare_incremental(pdf: &[u8]) -> Result<Vec<u8>, PdfSignError> {
    let mut inc = IncrementalDocument::load_from(std::io::Cursor::new(pdf))
        .map_err(|err| PdfSignError::msg(format!("read pdf: {err}")))?;
    let page_id = inc
        .get_prev_documents()
        .get_pages()
        .into_values()
        .next()
        .ok_or_else(|| PdfSignError::msg("PDF has no pages"))?;
    let catalog_id = inc
        .get_prev_documents()
        .trailer
        .get(b"Root")
        .and_then(Object::as_reference)
        .map_err(|err| PdfSignError::msg(format!("PDF catalog: {err}")))?;

    inc.opt_clone_object_to_new_document(catalog_id)
        .map_err(|err| PdfSignError::msg(format!("clone catalog: {err}")))?;
    inc.opt_clone_object_to_new_document(page_id)
        .map_err(|err| PdfSignError::msg(format!("clone page: {err}")))?;

    let placeholder = vec![0u8; CONTENTS_LEN];
    let byte_range = vec![
        Object::Integer(BYTE_RANGE_PLACEHOLDER),
        Object::Integer(BYTE_RANGE_PLACEHOLDER),
        Object::Integer(BYTE_RANGE_PLACEHOLDER),
        Object::Integer(BYTE_RANGE_PLACEHOLDER),
    ];
    let mut sig = Dictionary::new();
    sig.set("Type", "Sig");
    sig.set("Filter", "Adobe.PPKLite");
    sig.set("SubFilter", "adbe.pkcs7.detached");
    sig.set("ByteRange", Object::Array(byte_range));
    sig.set(
        "Contents",
        Object::String(placeholder, StringFormat::Hexadecimal),
    );
    sig.set("Reason", "Lariv");
    let sig_id = inc.new_document.add_object(Object::Dictionary(sig));

    let mut field = Dictionary::new();
    field.set("FT", "Sig");
    field.set("Type", "Annot");
    field.set("Subtype", "Widget");
    field.set("T", "Signature1");
    field.set("V", Object::Reference(sig_id));
    field.set(
        "Rect",
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(0),
        ]),
    );
    field.set("F", 132_i64);
    field.set("P", Object::Reference(page_id));
    let field_id = inc.new_document.add_object(Object::Dictionary(field));

    let mut form = Dictionary::new();
    form.set("SigFlags", 3_i64);
    form.set("Fields", Object::Array(vec![Object::Reference(field_id)]));
    let form_id = inc.new_document.add_object(Object::Dictionary(form));

    let existing_annots = existing_annots(&inc, page_id)?;
    {
        let page = inc
            .new_document
            .get_object_mut(page_id)
            .and_then(Object::as_dict_mut)
            .map_err(|err| PdfSignError::msg(format!("update page: {err}")))?;
        let mut annots = existing_annots;
        annots.push(Object::Reference(field_id));
        page.set("Annots", Object::Array(annots));
    }
    {
        let catalog = inc
            .new_document
            .get_object_mut(catalog_id)
            .and_then(Object::as_dict_mut)
            .map_err(|err| PdfSignError::msg(format!("update catalog: {err}")))?;
        catalog.set("AcroForm", Object::Reference(form_id));
    }

    let mut out = Vec::new();
    inc.save_to(&mut out)
        .map_err(|err| PdfSignError::msg(format!("write signed pdf: {err}")))?;
    Ok(out)
}

fn existing_annots(
    inc: &IncrementalDocument,
    page_id: ObjectId,
) -> Result<Vec<Object>, PdfSignError> {
    let page = inc
        .new_document
        .get_object(page_id)
        .and_then(Object::as_dict)
        .map_err(|err| PdfSignError::msg(format!("page dictionary: {err}")))?;
    match page.get(b"Annots") {
        Ok(Object::Array(items)) => Ok(items.clone()),
        Ok(Object::Reference(id)) => inc
            .get_prev_documents()
            .get_object(*id)
            .and_then(Object::as_array)
            .cloned()
            .or(Ok(Vec::new())),
        _ => Ok(Vec::new()),
    }
}

fn contents_marker() -> Vec<u8> {
    let mut marker = Vec::with_capacity(CONTENTS_LEN.saturating_mul(2).saturating_add(2));
    marker.push(b'<');
    marker.extend(std::iter::repeat_n(b'0', CONTENTS_LEN.saturating_mul(2)));
    marker.push(b'>');
    marker
}

fn byte_range_marker() -> String {
    let number = BYTE_RANGE_PLACEHOLDER.to_string();
    format!("{number} {number} {number} {number}")
}

fn find_slice(haystack: &[u8], needle: &[u8]) -> Result<usize, PdfSignError> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
        .ok_or_else(|| PdfSignError::msg("signature placeholder was not written"))
}

fn patch_byte_range(pdf: &mut [u8]) -> Result<(), PdfSignError> {
    let marker = byte_range_marker();
    let marker_bytes = marker.as_bytes();
    let at = find_slice(pdf, marker_bytes)?;
    let contents = contents_marker();
    let contents_at = find_slice(pdf, &contents)?;
    let before = contents_at;
    let after = contents_at
        .checked_add(contents.len())
        .ok_or_else(|| PdfSignError::msg("pdf offset overflow"))?;
    let file_len = pdf.len();
    let after_len = file_len
        .checked_sub(after)
        .ok_or_else(|| PdfSignError::msg("pdf offset overflow"))?;
    let numbers = [0_u64, before as u64, after as u64, after_len as u64];
    let mut replacement = String::new();
    for (index, number) in numbers.iter().enumerate() {
        let text = format!("{number:0width$}", width = BYTE_RANGE_WIDTH);
        if text.len() != BYTE_RANGE_WIDTH {
            return Err(PdfSignError::msg("pdf is too large to sign"));
        }
        if index > 0 {
            replacement.push(' ');
        }
        replacement.push_str(&text);
    }
    if replacement.len() != marker_bytes.len() {
        return Err(PdfSignError::msg("byte range placeholder width mismatch"));
    }
    let (_, rest) = pdf
        .split_at_mut_checked(at)
        .ok_or_else(|| PdfSignError::msg("byte range placeholder is out of range"))?;
    let (slot, _) = rest
        .split_at_mut_checked(marker_bytes.len())
        .ok_or_else(|| PdfSignError::msg("byte range placeholder is out of range"))?;
    slot.copy_from_slice(replacement.as_bytes());
    Ok(())
}

fn byte_ranges(pdf: &[u8]) -> Result<(&[u8], &[u8]), PdfSignError> {
    let contents = contents_marker();
    let contents_at = find_slice(pdf, &contents)?;
    let after = contents_at
        .checked_add(contents.len())
        .ok_or_else(|| PdfSignError::msg("pdf offset overflow"))?;
    let (head, rest) = pdf
        .split_at_checked(contents_at)
        .ok_or_else(|| PdfSignError::msg("contents placeholder is out of range"))?;
    let tail_at = after.saturating_sub(contents_at);
    let tail = rest
        .get(tail_at..)
        .ok_or_else(|| PdfSignError::msg("contents placeholder is out of range"))?;
    Ok((head, tail))
}

fn embed_cms(pdf: &mut [u8], cms: &[u8]) -> Result<(), PdfSignError> {
    let hex = hex_encode(cms);
    if hex.len() > CONTENTS_LEN.saturating_mul(2) {
        return Err(PdfSignError::msg("signature does not fit in the PDF"));
    }
    let mut padded = hex;
    padded.extend(std::iter::repeat_n(
        '0',
        CONTENTS_LEN.saturating_mul(2) - padded.len(),
    ));
    let contents = contents_marker();
    let contents_at = find_slice(pdf, &contents)?;
    let hex_at = contents_at.saturating_add(1);
    let (_, rest) = pdf
        .split_at_mut_checked(hex_at)
        .ok_or_else(|| PdfSignError::msg("contents placeholder is out of range"))?;
    let (slot, _) = rest
        .split_at_mut_checked(padded.len())
        .ok_or_else(|| PdfSignError::msg("contents placeholder is out of range"))?;
    slot.copy_from_slice(padded.as_bytes());
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        let hi = usize::from(byte >> 4);
        let lo = usize::from(byte & 0x0f);
        out.push(char::from(HEX.get(hi).copied().unwrap_or(b'0')));
        out.push(char::from(HEX.get(lo).copied().unwrap_or(b'0')));
    }
    out
}

async fn build_cms(
    pdf_digest: &[u8],
    backend: &dyn KeyBackend,
    key_ref: &str,
    identity: &super::cert::SignerIdentity,
) -> Result<Vec<u8>, PdfSignError> {
    let cert_der = backend.certificate_der(key_ref, identity).await?;
    let cert = Certificate::from_der(&cert_der)
        .map_err(|err| PdfSignError::msg(format!("signer certificate: {err}")))?;
    let signed_attrs = signed_attributes(pdf_digest)?;
    let attr_der = signed_attrs
        .to_der()
        .map_err(|err| PdfSignError::msg(format!("signed attributes: {err}")))?;
    let attr_digest = Sha256::digest(&attr_der);
    let signature = backend.sign_digest(key_ref, attr_digest.as_slice()).await?;
    let signature = SignatureValue::new(signature)
        .map_err(|err| PdfSignError::msg(format!("signature value: {err}")))?;

    let mut digest_algorithms = DigestAlgorithmIdentifiers::new();
    digest_algorithms
        .insert(sha256_alg()?)
        .map_err(|err| PdfSignError::msg(format!("digest algorithm: {err}")))?;

    let mut certificates = CertificateSet(SetOfVec::new());
    certificates
        .0
        .insert(CertificateChoices::Certificate(cert.clone()))
        .map_err(|err| PdfSignError::msg(format!("signer certificate set: {err}")))?;

    let sid = SignerIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
        issuer: cert.tbs_certificate.issuer.clone(),
        serial_number: cert.tbs_certificate.serial_number.clone(),
    });
    let signer = SignerInfo {
        version: CmsVersion::V1,
        sid,
        digest_alg: sha256_alg()?,
        signed_attrs: Some(signed_attrs),
        signature_algorithm: AlgorithmIdentifierOwned {
            oid: ECDSA_WITH_SHA_256,
            parameters: None,
        },
        signature,
        unsigned_attrs: None,
    };
    let mut signer_infos = SignerInfos(SetOfVec::new());
    signer_infos
        .0
        .insert(signer)
        .map_err(|err| PdfSignError::msg(format!("signer info: {err}")))?;

    let signed_data = SignedData {
        version: CmsVersion::V1,
        digest_algorithms,
        encap_content_info: EncapsulatedContentInfo {
            econtent_type: ID_DATA,
            econtent: None,
        },
        certificates: Some(certificates),
        crls: None,
        signer_infos,
    };
    let signed_der = signed_data
        .to_der()
        .map_err(|err| PdfSignError::msg(format!("signed data: {err}")))?;
    let content = Any::from_der(&signed_der)
        .map_err(|err| PdfSignError::msg(format!("signed data: {err}")))?;
    let info = ContentInfo {
        content_type: ID_SIGNED_DATA,
        content,
    };
    info.to_der()
        .map_err(|err| PdfSignError::msg(format!("cms: {err}")))
}

fn signed_attributes(pdf_digest: &[u8]) -> Result<Attributes, PdfSignError> {
    let mut attrs = Attributes::new();
    attrs
        .insert(attribute(ID_CONTENT_TYPE, &ID_DATA)?)
        .map_err(|err| PdfSignError::msg(format!("content type attribute: {err}")))?;
    let digest = OctetString::new(pdf_digest.to_vec())
        .map_err(|err| PdfSignError::msg(format!("message digest: {err}")))?;
    attrs
        .insert(attribute(ID_MESSAGE_DIGEST, &digest)?)
        .map_err(|err| PdfSignError::msg(format!("message digest attribute: {err}")))?;
    Ok(attrs)
}

fn attribute(
    oid: const_oid::ObjectIdentifier,
    value: &impl Encode,
) -> Result<Attribute, PdfSignError> {
    let encoded = value
        .to_der()
        .map_err(|err| PdfSignError::msg(format!("attribute value: {err}")))?;
    let any = Any::from_der(&encoded)
        .map_err(|err| PdfSignError::msg(format!("attribute value: {err}")))?;
    let mut values = SetOfVec::new();
    values
        .insert(any)
        .map_err(|err| PdfSignError::msg(format!("attribute value: {err}")))?;
    Ok(Attribute { oid, values })
}

fn sha256_alg() -> Result<AlgorithmIdentifierOwned, PdfSignError> {
    let null = Any::from_der(
        &Null
            .to_der()
            .map_err(|err| PdfSignError::msg(err.to_string()))?,
    )
    .map_err(|err| PdfSignError::msg(err.to_string()))?;
    Ok(AlgorithmIdentifierOwned {
        oid: ID_SHA_256,
        parameters: Some(null),
    })
}
