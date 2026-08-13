use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rand::RngCore;
use tracing::{info, warn};

use sea_orm::DatabaseConnection;

use super::adapters::msg91::{FlowRecipient, Msg91Client};
use super::adapters::smtp::send_otp_email;
use super::error::OtpError;
use super::preferences::{extra_fields, load_preferences};

pub const OTP_CACHE_PREFIX_PHONE: &str = "otp:phone:";
pub const OTP_CACHE_PREFIX_EMAIL: &str = "otp:email:";
pub const OTP_EXPIRY_SECS: u64 = 300;

struct CacheEntry {
    otp: String,
    expires_at: Instant,
}

/// Concurrency-safe in-memory OTP store.
pub struct MemoryCache {
    store: Mutex<HashMap<String, CacheEntry>>,
}

impl MemoryCache {
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }

    pub fn set(&self, key: String, otp: String, duration: Duration) {
        let Ok(mut guard) = self.store.lock() else {
            return;
        };
        guard.insert(
            key,
            CacheEntry {
                otp,
                expires_at: Instant::now() + duration,
            },
        );
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let Ok(mut guard) = self.store.lock() else {
            return None;
        };
        match guard.get(key) {
            Some(entry) if Instant::now() <= entry.expires_at => Some(entry.otp.clone()),
            Some(_) => {
                guard.remove(key);
                None
            }
            None => None,
        }
    }

    pub fn delete(&self, key: &str) {
        if let Ok(mut guard) = self.store.lock() {
            guard.remove(key);
        }
    }
}

impl Default for MemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a 6-digit numeric OTP.
pub fn generate_otp() -> String {
    let mut b = [0u8; 3];
    rand::thread_rng().fill_bytes(&mut b);
    let val = (u32::from(b[0]) | (u32::from(b[1]) << 8) | (u32::from(b[2]) << 16)) % 1_000_000;
    format!("{val:06}")
}

fn phone_to_cache_suffix(phone: &str) -> String {
    phone.trim().trim_start_matches('+').to_string()
}

fn cache_key_phone(phone: &str) -> String {
    format!("{OTP_CACHE_PREFIX_PHONE}{}", phone_to_cache_suffix(phone))
}

fn cache_key_email(email: &str) -> String {
    let s: String = email
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '@' | '.' | '_' | '+' | '-'))
        .collect();
    format!("{OTP_CACHE_PREFIX_EMAIL}{s}")
}

pub fn store_otp_phone(cache: &MemoryCache, phone: &str, otp: &str) {
    cache.set(
        cache_key_phone(phone),
        otp.to_string(),
        Duration::from_secs(OTP_EXPIRY_SECS),
    );
    info!(phone, "OTP stored for phone");
}

pub fn store_otp_email(cache: &MemoryCache, email: &str, otp: &str) {
    cache.set(
        cache_key_email(email),
        otp.to_string(),
        Duration::from_secs(OTP_EXPIRY_SECS),
    );
    info!(email, "OTP stored for email");
}

/// Verify OTP for an email or phone identifier (single-use on success).
pub fn verify_otp(cache: &MemoryCache, identifier: &str, otp: &str) -> bool {
    let identifier = identifier.trim();
    let key = if identifier.contains('@') {
        cache_key_email(identifier)
    } else {
        cache_key_phone(identifier)
    };

    match cache.get(&key) {
        Some(stored) if stored == otp => {
            cache.delete(&key);
            info!(identifier, "OTP verified");
            true
        }
        Some(_) => {
            warn!(identifier, "OTP mismatch");
            false
        }
        None => {
            warn!(identifier, "No OTP found");
            false
        }
    }
}

/// Generate, store, and send an SMS OTP via MSG91.
pub async fn send_sms_otp(
    db: &DatabaseConnection,
    cache: &MemoryCache,
    phone: &str,
) -> Result<(), OtpError> {
    let prefs = load_preferences(db).await?;

    let template_id = if !prefs.sms_otp_template_id.is_empty() {
        prefs.sms_otp_template_id.clone()
    } else {
        prefs.otp_template_id.clone()
    };
    if template_id.is_empty() {
        warn!("SMS_OTP_TEMPLATE_ID or OTP_TEMPLATE_ID not configured");
        return Err(OtpError::SendFailed);
    }
    if prefs.msg91_auth_key.is_empty() {
        warn!("MSG91_AUTH_KEY not configured");
        return Err(OtpError::SendFailed);
    }

    let otp = generate_otp();
    store_otp_phone(cache, phone, &otp);

    let otp_field = if prefs.sms_otp_field_name.is_empty() {
        "otp".to_string()
    } else {
        prefs.sms_otp_field_name.clone()
    };

    let mut recipient = FlowRecipient::new();
    recipient.insert(
        "mobiles".into(),
        serde_json::Value::String(phone_to_cache_suffix(phone)),
    );
    recipient.insert(otp_field, serde_json::Value::String(otp));
    for (k, v) in extra_fields(&prefs) {
        recipient.insert(k, v);
    }

    let client = Msg91Client::new(prefs.msg91_auth_key);
    match client
        .send_sms_flow(&template_id, vec![recipient], true)
        .await
    {
        Ok(res) => {
            info!(phone, ?res, "OTP SMS sent");
            Ok(())
        }
        Err(e) => {
            warn!(phone, error = %e, "Failed to send SMS OTP");
            Err(OtpError::SendFailed)
        }
    }
}

/// Generate, store, and send an email OTP via SMTP.
pub async fn send_email_otp(
    db: &DatabaseConnection,
    cache: &MemoryCache,
    email: &str,
) -> Result<(), OtpError> {
    let prefs = load_preferences(db).await?;

    if prefs.email_otp_template_string.is_empty() {
        warn!("EMAIL_OTP_TEMPLATE_STRING not configured");
        return Err(OtpError::SendFailed);
    }
    if prefs.smtp_host.is_empty() || prefs.smtp_from.is_empty() {
        warn!("SMTP not configured (host and from are required)");
        return Err(OtpError::SendFailed);
    }

    let otp = generate_otp();
    store_otp_email(cache, email, &otp);

    let body = prefs.email_otp_template_string.replace("$otp", &otp);
    match send_otp_email(&prefs, email, &body).await {
        Ok(()) => {
            info!(email, "OTP email sent");
            Ok(())
        }
        Err(e) => {
            warn!(email, error = %e, "Failed to send email OTP");
            Err(OtpError::SendFailed)
        }
    }
}
