use scrypt::{Params, scrypt};
use subtle::ConstantTimeEq;

use crate::plugins::users::error::UsersError;

// Scrypt params matching Go: N=32768, r=8, p=1, keyLen=32.
const LOG_N: u8 = 15; // 2^15 = 32768
const R: u32 = 8;
const P: u32 = 1;
const KEY_LEN: usize = 32;
pub const SALT_LEN: usize = 256;

// Hash password with the given salt using raw scrypt (same as golang.org/x/crypto/scrypt).
pub fn hash_password(password: &[u8], salt: &[u8]) -> Result<Vec<u8>, UsersError> {
    let params = Params::new(LOG_N, R, P).map_err(|e| UsersError::Crypto(e.to_string()))?;
    let mut output = vec![0u8; KEY_LEN];
    scrypt(password, salt, &params, &mut output).map_err(|e| UsersError::Crypto(e.to_string()))?;
    Ok(output)
}

pub fn generate_salt() -> Vec<u8> {
    let mut salt = vec![0u8; SALT_LEN];
    rand::fill(&mut salt);
    salt
}

pub fn verify_password(
    password: &[u8],
    salt: &[u8],
    expected_hash: &[u8],
) -> Result<bool, UsersError> {
    let got = hash_password(password, salt)?;
    Ok(got.ct_eq(expected_hash).into())
}
