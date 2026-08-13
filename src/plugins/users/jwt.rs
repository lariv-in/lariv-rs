use base64::{Engine, engine::general_purpose::STANDARD as B64};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::plugins::users::{entities::User, error::UsersError};

const ISSUER: &str = "lariv";

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub sub: String,
    pub aud: Vec<String>,
    pub exp: i64,
    pub iat: i64,
    pub nbf: i64,
    pub jti: String,
}

pub fn audience(jwt_issuer: &[u8]) -> String {
    format!("lariv-{}", B64.encode(jwt_issuer))
}

pub fn subject(user: &User) -> String {
    format!("{}-{}", user.id, B64.encode(&user.password_salt))
}

pub fn issue_token(
    user: &User,
    signing_key: &[u8],
    jwt_issuer: &[u8],
    ttl: Duration,
) -> Result<String, UsersError> {
    let now = Utc::now();
    let exp = now + ttl;
    let claims = Claims {
        iss: ISSUER.to_string(),
        sub: subject(user),
        aud: vec![audience(jwt_issuer)],
        exp: exp.timestamp(),
        iat: now.timestamp(),
        nbf: now.timestamp(),
        jti: Uuid::new_v4().to_string(),
    };
    let header = Header::new(Algorithm::HS512);
    Ok(encode(
        &header,
        &claims,
        &EncodingKey::from_secret(signing_key),
    )?)
}

pub fn parse_token(
    token: &str,
    signing_key: &[u8],
    jwt_issuer: &[u8],
) -> Result<Claims, UsersError> {
    let mut validation = Validation::new(Algorithm::HS512);
    validation.set_issuer(&[ISSUER]);
    validation.set_audience(&[audience(jwt_issuer)]);
    validation.leeway = 24 * 60 * 60; // 24h, matching Go
    validation.validate_nbf = true;

    let data = decode::<Claims>(token, &DecodingKey::from_secret(signing_key), &validation)?;
    Ok(data.claims)
}

pub fn user_id_from_subject(sub: &str) -> Result<i64, UsersError> {
    let id_str = sub.split('-').next().ok_or(UsersError::AuthFailed)?;
    id_str.parse().map_err(|_| UsersError::AuthFailed)
}
