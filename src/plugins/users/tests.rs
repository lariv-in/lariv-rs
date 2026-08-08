#[cfg(test)]
mod tests {
    use crate::plugins::users::{jwt, password};
    use chrono::Duration;

    #[test]
    fn scrypt_roundtrip() {
        let salt = password::generate_salt();
        assert_eq!(salt.len(), password::SALT_LEN);
        let hash = password::hash_password(b"secret", &salt).unwrap();
        assert!(password::verify_password(b"secret", &salt, &hash).unwrap());
        assert!(!password::verify_password(b"wrong", &salt, &hash).unwrap());
    }

    #[test]
    fn jwt_subject_binds_salt() {
        use crate::plugins::users::entities::User;
        use chrono::Utc;

        let user = User {
            id: 7,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            name: "A".into(),
            email: "a@b.c".into(),
            phone: "1".into(),
            is_superuser: false,
            role_id: 1,
            password_hash: vec![1, 2, 3],
            password_salt: vec![9; 32],
            timezone: "Asia/Kolkata".into(),
        };
        let key = vec![0u8; 64];
        let issuer = vec![1u8; 64];
        let token = jwt::issue_token(&user, &key, &issuer, Duration::hours(1)).unwrap();
        let claims = jwt::parse_token(&token, &key, &issuer).unwrap();
        assert_eq!(claims.sub, jwt::subject(&user));
        assert_eq!(jwt::user_id_from_subject(&claims.sub).unwrap(), 7);

        let mut changed = user.clone();
        changed.password_salt = vec![8; 32];
        assert_ne!(claims.sub, jwt::subject(&changed));
    }
}
