//! Employee ID card: Minijinja template → Typst → PDF.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use jsonwebtoken::jwk::{Jwk, JwkSet};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use minijinja::{Environment, UndefinedBehavior};
use sea_orm::DatabaseConnection;
use serde::Serialize;

use lariv_core::datetime::{format_date, format_time};
use lariv_core::typst;
use lariv_plugin_filesystem::state::FilesystemState;
use lariv_plugin_signing::backend::PemKeyBackend;
use lariv_plugin_signing::cert::SignerIdentity;
use lariv_plugin_signing::pdf::sign_pdf;

use crate::entities::employee;
use crate::forms::{ACCOUNT_TYPE_CHOICES, MARITAL_STATUS_CHOICES};
use crate::id_card_assets::VnodeImageContext;
use crate::id_card_template::DEFAULT_ID_CARD_TEMPLATE;
use crate::logic::preferences::{ensure_id_card_signing_key, load_id_card_format};
use crate::logic::user::{user_label, user_role_title};
use crate::scope::format_timestamp;

const QR_FILENAME: &str = "id-card-qr.svg";
const LOGO_FILENAME: &str = "kds-tagore-logo.png";
const LOGO_PNG: &[u8] = include_bytes!("../templates/kds-tagore-logo.png");
const ID_CARD_SIGNER: &str = "Lariv HR";
const SIGNING_CERT_SECS: u64 = 60 * 60 * 24 * 365 * 10;

pub struct IdCardPdf {
    pub bytes: Vec<u8>,
    pub filename_base: String,
}

/// One employee, with every stored field available to the ID card template.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct IdCardContext {
    #[serde(rename = "ID")]
    pub id: i64,
    pub employee_code: String,
    pub user_id: i64,
    pub manager_id: Option<i64>,
    pub manager: String,
    pub name: String,
    /// Display title of the employee's user role.
    pub role: String,
    pub mobile: String,
    pub email: String,
    pub hired_at: String,
    pub hired_at_display: String,
    pub is_probationary: bool,
    pub verified: bool,
    pub created_at: String,
    pub created_at_display: String,
    pub updated_at: String,
    pub updated_at_display: String,
    pub fathers_name: String,
    pub date_of_birth: String,
    pub gender: String,
    pub gender_code: String,
    pub marital_status: String,
    pub marital_status_code: String,
    pub nationality: String,
    pub is_disabled: Option<bool>,
    pub is_disabled_label: String,
    pub disability_type: String,
    pub photograph_vnode_id: Option<i64>,
    pub blood_group: String,
    pub identification_mark: String,
    pub present_address: String,
    pub present_pin_code: String,
    pub permanent_address: String,
    pub permanent_pin_code: String,
    pub emergency_contact_name: String,
    pub emergency_contact_relation: String,
    pub emergency_contact_mobile: String,
    pub aadhar_vnode_id: Option<i64>,
    pub pan_vnode_id: Option<i64>,
    pub passport_vnode_id: Option<i64>,
    pub account_holder_name: String,
    pub account_number: String,
    pub account_ifsc_code: String,
    pub account_type: String,
    pub account_type_code: String,
    pub qualifications: String,
    pub date_of_joining: String,
    pub probation_end_date: String,
    pub work_start: String,
    pub work_end: String,
    pub base_salary: String,
    pub hourly_wage: String,
    /// Filename of an SVG QR code in the Typst work directory. Empty when no signing key is set.
    pub qr_code: String,
}

pub async fn id_card_context(
    db: &DatabaseConnection,
    employee: &employee::Model,
    tz: &str,
) -> IdCardContext {
    let (created_at, created_at_display) = opt_timestamp(employee.created_at, tz);
    let (updated_at, updated_at_display) = opt_timestamp(employee.updated_at, tz);
    IdCardContext {
        id: employee.id,
        employee_code: employee.employee_code.clone(),
        user_id: employee.user_id,
        manager_id: employee.manager_id.filter(|id| *id > 0),
        manager: user_label(db, employee.manager_id).await,
        name: text(&employee.name),
        role: user_role_title(db, employee.user_id).await,
        mobile: text(&employee.mobile),
        email: text(&employee.email),
        hired_at: employee.hired_at.to_rfc3339(),
        hired_at_display: format_timestamp(employee.hired_at, tz),
        is_probationary: employee.is_probationary,
        verified: employee.verified,
        created_at,
        created_at_display,
        updated_at,
        updated_at_display,
        fathers_name: text(&employee.fathers_name),
        date_of_birth: employee.date_of_birth.map(format_date).unwrap_or_default(),
        gender: employee
            .gender
            .map(|gender| gender.label().to_string())
            .unwrap_or_default(),
        gender_code: employee
            .gender
            .map(|gender| gender.as_str().to_string())
            .unwrap_or_default(),
        marital_status: choice_label(MARITAL_STATUS_CHOICES, employee.marital_status.as_deref()),
        marital_status_code: text(&employee.marital_status),
        nationality: text(&employee.nationality),
        is_disabled: employee.is_disabled,
        is_disabled_label: match employee.is_disabled {
            Some(true) => "Yes".to_string(),
            Some(false) => "No".to_string(),
            None => String::new(),
        },
        disability_type: text(&employee.disability_type),
        photograph_vnode_id: positive_id(employee.photograph_vnode_id),
        blood_group: employee
            .blood_group
            .map(|group| group.label().to_string())
            .unwrap_or_default(),
        identification_mark: text(&employee.identification_mark),
        present_address: text(&employee.present_address),
        present_pin_code: text(&employee.present_pin_code),
        permanent_address: text(&employee.permanent_address),
        permanent_pin_code: text(&employee.permanent_pin_code),
        emergency_contact_name: text(&employee.emergency_contact_name),
        emergency_contact_relation: text(&employee.emergency_contact_relation),
        emergency_contact_mobile: text(&employee.emergency_contact_mobile),
        aadhar_vnode_id: positive_id(employee.aadhar_vnode_id),
        pan_vnode_id: positive_id(employee.pan_vnode_id),
        passport_vnode_id: positive_id(employee.passport_vnode_id),
        account_holder_name: text(&employee.account_holder_name),
        account_number: text(&employee.account_number),
        account_ifsc_code: text(&employee.account_ifsc_code),
        account_type: choice_label(ACCOUNT_TYPE_CHOICES, employee.account_type.as_deref()),
        account_type_code: text(&employee.account_type),
        qualifications: text(&employee.qualifications),
        date_of_joining: employee
            .date_of_joining
            .map(format_date)
            .unwrap_or_default(),
        probation_end_date: employee
            .probation_end_date
            .map(format_date)
            .unwrap_or_default(),
        work_start: employee.work_start.map(format_time).unwrap_or_default(),
        work_end: employee.work_end.map(format_time).unwrap_or_default(),
        base_salary: money(employee.base_salary),
        hourly_wage: money(employee.hourly_wage),
        qr_code: String::new(),
    }
}

/// Compile `template` with sample employee data. A blank template uses the shipped card.
pub async fn render_id_card_preview(
    fs: &FilesystemState,
    template_src: Option<&str>,
    signing_key: Option<&str>,
) -> Result<Vec<u8>, String> {
    let template = template_src
        .map(str::trim)
        .filter(|template| !template.is_empty())
        .unwrap_or(DEFAULT_ID_CARD_TEMPLATE);
    compile_id_card(fs, template, &sample_id_card_context(), signing_key).await
}

/// Render the saved ID card template for `employee`. A blank preference uses the shipped card.
pub async fn render_employee_id_card(
    fs: &FilesystemState,
    employee: &employee::Model,
    tz: &str,
) -> Result<IdCardPdf, String> {
    let saved = load_id_card_format(&fs.db).await?;
    let template = saved.trim();
    let template = if template.is_empty() {
        DEFAULT_ID_CARD_TEMPLATE
    } else {
        template
    };
    let signing_key = ensure_id_card_signing_key(&fs.db).await?;
    let context = id_card_context(&fs.db, employee, tz).await;
    let bytes = compile_id_card(fs, template, &context, Some(signing_key.as_str())).await?;
    Ok(IdCardPdf {
        bytes,
        filename_base: filename_base(employee),
    })
}

async fn compile_id_card(
    fs: &FilesystemState,
    template: &str,
    context: &IdCardContext,
    signing_key: Option<&str>,
) -> Result<Vec<u8>, String> {
    let work_dir = typst::typst_work_dir();
    let result = compile_id_card_in(fs, &work_dir, template, context, signing_key).await;
    if let Err(err) = std::fs::remove_dir_all(&work_dir)
        && err.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(error = %err, path = %work_dir.display(), "failed to remove id card work dir");
    }
    result
}

async fn compile_id_card_in(
    fs: &FilesystemState,
    work_dir: &Path,
    template: &str,
    context: &IdCardContext,
    signing_key: Option<&str>,
) -> Result<Vec<u8>, String> {
    std::fs::create_dir_all(work_dir).map_err(|err| format!("create id card work dir: {err}"))?;
    write_id_card_logo(work_dir)?;
    let signing_key = signing_key.map(str::trim).filter(|key| !key.is_empty());
    let mut context = context.clone();
    if let Some(pem) = signing_key {
        let jwt = id_card_jwt(&context, pem)?;
        write_qr_svg(work_dir, &jwt)?;
        context.qr_code = QR_FILENAME.to_string();
    }
    let vnode_ctx =
        VnodeImageContext::new(fs.db.clone(), Arc::clone(&fs.store), work_dir.to_path_buf());
    let typst_src = render_template(template, &context, Some(&vnode_ctx))?;
    let pdf_bytes = typst::typst_compile_in(work_dir, &typst_src)
        .await
        .map_err(|err| format!("Typst compile failed:\n{err}"))?;
    if let Some(pem) = signing_key {
        sign_id_card_pdf(&pdf_bytes, pem).await
    } else {
        Ok(pdf_bytes)
    }
}

/// ES256 JWT of the card context, excluding `QrCode`, with the public key in the header.
fn id_card_jwt(context: &IdCardContext, pem: &str) -> Result<String, String> {
    let mut claims =
        serde_json::to_value(context).map_err(|err| format!("serialize ID card context: {err}"))?;
    if let Some(object) = claims.as_object_mut() {
        object.remove("QrCode");
    }
    let key = EncodingKey::from_ec_pem(pem.as_bytes())
        .map_err(|err| format!("ID card signing key: {err}"))?;
    let mut header = Header::new(Algorithm::ES256);
    header.jwk = Some(id_card_public_jwk(&key)?);
    encode(&header, &claims, &key).map_err(|err| format!("sign ID card JWT: {err}"))
}

/// Public JWK for a PKCS#8 PEM P-256 private key.
pub fn id_card_public_jwk(key: &EncodingKey) -> Result<Jwk, String> {
    Jwk::from_encoding_key(key, Algorithm::ES256)
        .map_err(|err| format!("ID card public key: {err}"))
}

/// JWKS document for the stored ID card signing key. Generates a key when none is stored.
pub async fn id_card_jwks_json(db: &DatabaseConnection) -> Result<String, String> {
    let pem = ensure_id_card_signing_key(db).await?;
    let key = EncodingKey::from_ec_pem(pem.as_bytes())
        .map_err(|err| format!("ID card signing key: {err}"))?;
    let set = JwkSet {
        keys: vec![id_card_public_jwk(&key)?],
    };
    serde_json::to_string(&set).map_err(|err| format!("encode ID card public keys: {err}"))
}

fn write_id_card_logo(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|err| format!("create id card work dir: {err}"))?;
    std::fs::write(dir.join(LOGO_FILENAME), LOGO_PNG)
        .map_err(|err| format!("write ID card logo: {err}"))
}

fn write_qr_svg(dir: &Path, jwt: &str) -> Result<(), String> {
    let code =
        qrcode::QrCode::new(jwt.as_bytes()).map_err(|err| format!("ID card QR code: {err}"))?;
    let svg = code.render::<qrcode::render::svg::Color<'_>>().build();
    std::fs::write(dir.join(QR_FILENAME), svg)
        .map_err(|err| format!("write ID card QR code: {err}"))
}

async fn sign_id_card_pdf(pdf: &[u8], pem: &str) -> Result<Vec<u8>, String> {
    let backend = PemKeyBackend::parse(pem).map_err(|err| format!("ID card signing key: {err}"))?;
    let identity = SignerIdentity {
        authority_name: ID_CARD_SIGNER.to_string(),
        validity: Duration::from_secs(SIGNING_CERT_SECS),
    };
    sign_pdf(pdf, &backend, "id-card", &identity)
        .await
        .map_err(|err| format!("sign ID card PDF: {err}"))
}

fn render_template(
    template: &str,
    context: &IdCardContext,
    vnode_ctx: Option<&VnodeImageContext>,
) -> Result<String, String> {
    let value =
        serde_json::to_value(context).map_err(|err| format!("serialize ID card context: {err}"))?;
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::SemiStrict);
    env.add_filter("typst", typst_markup);
    if let Some(ctx) = vnode_ctx {
        let ctx = ctx.clone();
        env.add_function(
            "vnodeImage",
            move |vnode_id: i64| -> Result<String, minijinja::Error> {
                ctx.resolve_sync(vnode_id).map_err(|err| {
                    minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, err)
                })
            },
        );
    }
    let tmpl = env
        .template_from_str(template)
        .map_err(|err| format!("invalid ID card template: {err}"))?;
    tmpl.render(value)
        .map_err(|err| format!("rendering ID card template failed: {err}"))
}

/// Escape text for Typst markup.
fn typst_markup(value: String) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' | '#' | '[' | ']' | '*' | '_' | '`' | '$' | '@' | '<' | '>' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

fn filename_base(employee: &employee::Model) -> String {
    let slug: String = employee
        .name
        .as_deref()
        .unwrap_or("")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        format!("id-card-{}", employee.id)
    } else {
        format!("id-card-{}-{slug}", employee.id)
    }
}

fn text(value: &Option<String>) -> String {
    value.clone().unwrap_or_default()
}

fn positive_id(id: Option<i64>) -> Option<i64> {
    id.filter(|id| *id > 0)
}

fn money(value: Option<rust_decimal::Decimal>) -> String {
    value
        .map(|amount| amount.normalize().to_string())
        .unwrap_or_default()
}

fn choice_label(choices: &[(&str, &str)], value: Option<&str>) -> String {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return String::new();
    };
    choices
        .iter()
        .find(|(key, _)| *key == value)
        .map(|(_, label)| (*label).to_string())
        .unwrap_or_else(|| value.to_string())
}

fn sample_id_card_context() -> IdCardContext {
    IdCardContext {
        id: 4,
        employee_code: "202600009".into(),
        user_id: 9,
        manager_id: Some(2),
        manager: "Grace".into(),
        name: "Ada Lovelace".into(),
        role: "Operator".into(),
        mobile: "9990001111".into(),
        email: "ada@example.com".into(),
        hired_at: "2026-01-15T09:00:00+00:00".into(),
        hired_at_display: "15/01/2026 14:30".into(),
        is_probationary: false,
        verified: true,
        created_at: String::new(),
        created_at_display: String::new(),
        updated_at: String::new(),
        updated_at_display: String::new(),
        fathers_name: String::new(),
        date_of_birth: "10/12/1990".into(),
        gender: "Female".into(),
        gender_code: "female".into(),
        marital_status: String::new(),
        marital_status_code: String::new(),
        nationality: String::new(),
        is_disabled: None,
        is_disabled_label: String::new(),
        disability_type: String::new(),
        photograph_vnode_id: None,
        blood_group: "O+".into(),
        identification_mark: String::new(),
        present_address: String::new(),
        present_pin_code: String::new(),
        permanent_address: String::new(),
        permanent_pin_code: String::new(),
        emergency_contact_name: String::new(),
        emergency_contact_relation: String::new(),
        emergency_contact_mobile: String::new(),
        aadhar_vnode_id: None,
        pan_vnode_id: None,
        passport_vnode_id: None,
        account_holder_name: String::new(),
        account_number: String::new(),
        account_ifsc_code: String::new(),
        account_type: String::new(),
        account_type_code: String::new(),
        qualifications: String::new(),
        date_of_joining: "15/01/2026".into(),
        probation_end_date: String::new(),
        work_start: String::new(),
        work_end: String::new(),
        base_salary: String::new(),
        hourly_wage: String::new(),
        qr_code: String::new(),
    }
}

fn opt_timestamp(value: Option<DateTime<Utc>>, tz: &str) -> (String, String) {
    match value {
        Some(value) => (value.to_rfc3339(), format_timestamp(value, tz)),
        None => (String::new(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LOGO_FILENAME, QR_FILENAME, id_card_jwt, id_card_public_jwk, render_template,
        sample_id_card_context, sign_id_card_pdf, typst_markup, write_id_card_logo, write_qr_svg,
    };
    use crate::id_card_template::DEFAULT_ID_CARD_TEMPLATE;
    use crate::logic::preferences::generate_id_card_signing_key;
    use jsonwebtoken::jwk::JwkSet;
    use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Validation, decode};
    use lariv_core::typst;
    use p256::elliptic_curve::Generate;
    use p256::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};

    async fn compile_card(src: &str) -> Vec<u8> {
        let work_dir = typst::typst_work_dir();
        write_id_card_logo(&work_dir).expect("logo");
        let pdf = typst::typst_compile_in(&work_dir, src).await.expect("pdf");
        if let Err(err) = std::fs::remove_dir_all(&work_dir) {
            tracing::warn!(error = %err, "failed to remove id card test dir");
        }
        pdf
    }

    #[test]
    fn typst_filter_escapes_markup() {
        assert_eq!(typst_markup("A # B".into()), "A \\# B");
    }

    #[tokio::test]
    async fn default_id_card_template_renders() {
        let context = sample_id_card_context();
        let src = render_template(DEFAULT_ID_CARD_TEMPLATE, &context, None).expect("template");
        assert!(src.contains("Ada Lovelace"));
        assert!(src.contains("Operator"));
        assert!(src.contains("row(\"ID\", \"202600009\")"));
        assert!(src.contains(LOGO_FILENAME));
        assert!(!src.contains("vnodeImage"));
        let pdf = compile_card(&src).await;
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn default_id_card_escapes_employee_name() {
        let mut context = sample_id_card_context();
        context.name = "Ada # Lovelace".into();
        let src = render_template(DEFAULT_ID_CARD_TEMPLATE, &context, None).expect("template");
        assert!(src.contains("Ada \\# Lovelace"));
        let pdf = compile_card(&src).await;
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn photo_frame_accepts_clip() {
        let src = r#"
#set page(width: 40mm, height: 50mm, margin: 2mm)
#box(width: 20mm, height: 26mm, radius: 1.2mm, clip: true, stroke: 0.4pt + luma(180))[Photo]
"#;
        let pdf = typst::typst_compile(src).await.expect("pdf");
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn generated_signing_key_publishes_ec_jwk() {
        let pem = generate_id_card_signing_key().expect("key");
        let key = EncodingKey::from_ec_pem(pem.as_bytes()).expect("pem");
        let jwk = id_card_public_jwk(&key).expect("jwk");
        let set = JwkSet { keys: vec![jwk] };
        let json = serde_json::to_value(&set).expect("json");
        assert_eq!(
            json.pointer("/keys/0/kty").and_then(|v| v.as_str()),
            Some("EC")
        );
        assert_eq!(
            json.pointer("/keys/0/crv").and_then(|v| v.as_str()),
            Some("P-256")
        );
    }

    fn test_signing_pem() -> (String, String) {
        let key = p256::ecdsa::SigningKey::try_generate().expect("signing key");
        let private_pem = key
            .to_pkcs8_pem(LineEnding::LF)
            .expect("private pem")
            .to_string();
        let public_pem = key
            .verifying_key()
            .to_public_key_pem(LineEnding::LF)
            .expect("public pem");
        (private_pem, public_pem)
    }

    #[tokio::test]
    async fn signed_id_card_embeds_qr_jwt() {
        let (private_pem, public_pem) = test_signing_pem();
        let context = sample_id_card_context();
        let jwt = id_card_jwt(&context, &private_pem).expect("jwt");
        assert!(!jwt.contains(QR_FILENAME));

        let mut validation = Validation::new(Algorithm::ES256);
        validation.validate_exp = false;
        validation.required_spec_claims.clear();
        let data = decode::<serde_json::Value>(
            &jwt,
            &DecodingKey::from_ec_pem(public_pem.as_bytes()).expect("public key"),
            &validation,
        )
        .expect("verify jwt");
        assert!(data.header.jwk.is_some());
        assert!(data.claims.get("QrCode").is_none());
        assert_eq!(
            data.claims.get("Name").and_then(serde_json::Value::as_str),
            Some("Ada Lovelace")
        );

        let work_dir = typst::typst_work_dir();
        std::fs::create_dir_all(&work_dir).expect("work dir");
        write_qr_svg(&work_dir, &jwt).expect("qr");
        write_id_card_logo(&work_dir).expect("logo");
        let mut context = context;
        context.qr_code = QR_FILENAME.to_string();
        let src = render_template(DEFAULT_ID_CARD_TEMPLATE, &context, None).expect("template");
        assert!(src.contains(QR_FILENAME));
        let pdf = typst::typst_compile_in(&work_dir, &src).await.expect("pdf");
        let signed = sign_id_card_pdf(&pdf, &private_pem).await.expect("sign");
        assert!(signed.starts_with(b"%PDF"));
        assert!(
            signed
                .windows(b"Adobe.PPKLite".len())
                .any(|window| window == b"Adobe.PPKLite")
        );
        if let Err(err) = std::fs::remove_dir_all(&work_dir) {
            tracing::warn!(error = %err, "failed to remove signed id card test dir");
        }
    }
}
