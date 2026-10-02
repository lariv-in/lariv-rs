//! Signing scope and PDF signature tests.

use async_trait::async_trait;
use chrono::Utc;
use der::Decode;
use lariv_rs::plugins::signing::backend::kms::{KmsClient, KmsKeys};
use lariv_rs::plugins::signing::backend::{KeyBackend, KeyError};
use lariv_rs::plugins::signing::cert::SignerIdentity;
use lariv_rs::plugins::signing::entities::user_signature::{self, Entity as UserSignatureEntity};
use lariv_rs::plugins::signing::pdf::{is_pdf_bytes, sign_pdf};
use lariv_rs::plugins::signing::scope::{find_own_signature, scope_own};
use lariv_rs::plugins::users::entities::user::Model as User;
use lariv_rs::plugins::users::state::AuthContext;
use lopdf::{Document, Object};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, Database, DatabaseBackend, EntityTrait,
    Schema,
};

fn auth(id: i64, is_superuser: bool) -> AuthContext {
    AuthContext {
        user: User {
            id,
            created_at: None,
            updated_at: None,
            name: "Signer".into(),
            email: String::new().into(),
            phone: String::new().into(),
            is_superuser,
            role_id: 1,
            password_hash: None,
            password_salt: None,
            timezone: "UTC".into(),
        },
        role: "admin".into(),
        timezone: "UTC".into(),
    }
}

async fn memory_db() -> sea_orm::DatabaseConnection {
    let db = Database::connect("sqlite::memory:").await.expect("sqlite");
    let schema = Schema::new(DatabaseBackend::Sqlite);
    let stmt = schema.create_table_from_entity(UserSignatureEntity);
    db.execute(&stmt).await.expect("create user_signatures");
    db
}

#[tokio::test]
async fn superuser_cannot_read_another_users_signature() {
    let db = memory_db().await;
    let now = Utc::now();
    for (user_id, key_ref) in [(1_i64, "user-1.pem"), (2_i64, "user-2.pem")] {
        user_signature::ActiveModel {
            user_id: Set(user_id),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            key_ref: Set(key_ref.into()),
        }
        .insert(&db)
        .await
        .expect("insert signature");
    }

    let superuser = auth(1, true);
    let own = find_own_signature(&db, &superuser).await.expect("own row");
    assert_eq!(own.user_id, 1);
    assert_eq!(own.key_ref, "user-1.pem");

    let rows = scope_own(UserSignatureEntity::find(), &superuser)
        .all(&db)
        .await
        .expect("list own");
    assert_eq!(rows.len(), 1);
    let first = rows.first().expect("own row");
    assert_eq!(first.user_id, 1);
}

fn minimal_pdf() -> Vec<u8> {
    let mut doc = Document::with_version("1.4");
    let pages_id = doc.new_object_id();
    let mut page = lopdf::Dictionary::new();
    page.set("Type", "Page");
    page.set("Parent", Object::Reference(pages_id));
    page.set(
        "MediaBox",
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(612),
            Object::Integer(792),
        ]),
    );
    let page_id = doc.add_object(Object::Dictionary(page));
    let mut pages = lopdf::Dictionary::new();
    pages.set("Type", "Pages");
    pages.set("Count", 1_i64);
    pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    doc.set_object(pages_id, Object::Dictionary(pages));
    let mut catalog = lopdf::Dictionary::new();
    catalog.set("Type", "Catalog");
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));
    doc.trailer.set("Root", Object::Reference(catalog_id));
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).expect("save pdf");
    bytes
}

#[tokio::test]
async fn local_backend_signs_pdf_with_the_callers_key() {
    let dir = std::env::temp_dir().join(format!(
        "lariv-signing-local-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let keys = lariv_rs::plugins::signing::backend::local::LocalKeys::new(
        dir.to_string_lossy().into_owned(),
    );
    let first = keys.create_key(1).await.expect("key 1");
    let second = keys.create_key(2).await.expect("key 2");
    assert_ne!(first, second);

    let pdf = minimal_pdf();
    assert!(is_pdf_bytes(&pdf));
    let identity = SignerIdentity::from_preferences("Lariv", 365 * 24 * 60 * 60 * 1_000_000_000)
        .expect("identity");
    let signed_first = sign_pdf(&pdf, &keys, &first, &identity)
        .await
        .expect("sign 1");
    let signed_second = sign_pdf(&pdf, &keys, &second, &identity)
        .await
        .expect("sign 2");
    assert!(signed_first.starts_with(b"%PDF-"));
    assert!(signed_second.starts_with(b"%PDF-"));
    assert!(
        signed_first
            .windows(10)
            .any(|window| window == b"/ByteRange")
    );
    assert_ne!(signed_first, signed_second);

    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn local_certificate_uses_preferences_and_pdf_extensions() {
    let dir = std::env::temp_dir().join(format!(
        "lariv-signing-cert-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let keys = lariv_rs::plugins::signing::backend::local::LocalKeys::new(
        dir.to_string_lossy().into_owned(),
    );
    let key_ref = keys.create_key(3).await.expect("key");
    let identity = SignerIdentity::from_preferences("City Clerk", 2 * 24 * 60 * 60 * 1_000_000_000)
        .expect("identity");
    let der = keys
        .certificate_der(&key_ref, &identity)
        .await
        .expect("certificate");
    let cert = x509_cert::Certificate::from_der(&der).expect("parse certificate");
    let subject = cert.tbs_certificate.subject.to_string();
    assert!(subject.contains("City Clerk"), "{subject}");
    let extensions = cert
        .tbs_certificate
        .extensions
        .as_ref()
        .expect("extensions");
    let by_oid = |oid: &str| {
        extensions
            .iter()
            .find(|ext| ext.extn_id.to_string() == oid)
            .unwrap_or_else(|| panic!("missing {oid}"))
    };
    let basic = by_oid("2.5.29.19");
    assert!(!basic.critical);
    let key_id = by_oid("2.5.29.14");
    assert!(!key_id.critical);
    assert!(!key_id.extn_value.as_bytes().is_empty());
    let usage = by_oid("2.5.29.15");
    assert!(usage.critical);
    let extended = by_oid("2.5.29.37");
    assert!(!extended.critical);
    let extended_bytes = extended.extn_value.as_bytes();
    assert!(
        extended_bytes
            .windows(8)
            .any(|window| window == [0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x24]),
        "document signing EKU missing"
    );
    assert!(
        extended_bytes
            .windows(9)
            .any(|window| { window == [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x2f, 0x01, 0x01, 0x05] }),
        "adobe PDF signing EKU missing"
    );
    std::fs::remove_dir_all(&dir).ok();
}

struct FakeKms {
    signature: Vec<u8>,
}

#[async_trait]
impl KmsClient for FakeKms {
    async fn create_signing_key(&self, user_id: i64) -> Result<String, KeyError> {
        Ok(format!(
            "projects/test/keys/user-{user_id}/cryptoKeyVersions/1"
        ))
    }

    async fn sign_sha256(&self, _key_ref: &str, _digest: &[u8]) -> Result<Vec<u8>, KeyError> {
        Ok(self.signature.clone())
    }

    async fn public_key_pem(&self, _key_ref: &str) -> Result<String, KeyError> {
        Err(KeyError::msg("fake kms has no public key"))
    }
}

#[tokio::test]
async fn kms_backend_uses_the_client_trait() {
    let backend = KmsKeys::from_client(FakeKms {
        signature: b"der-sig".to_vec(),
    });
    let key_ref = backend.create_key(7).await.expect("create");
    assert!(key_ref.contains("user-7"));
    let signature = backend
        .sign_digest(&key_ref, &[1, 2, 3, 4])
        .await
        .expect("sign");
    assert_eq!(signature, b"der-sig");
}
