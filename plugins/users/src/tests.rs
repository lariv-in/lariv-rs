#[cfg(test)]
mod tests {
    use crate::{jwt, password};
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
        use crate::entities::User;
        use chrono::Utc;

        let user = User {
            id: 7,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            name: "A".into(),
            email: "a@b.c".into(),
            phone: "1".into(),
            role: "unassigned".into(),
            password_hash: Some(vec![1, 2, 3]),
            password_salt: Some(vec![9; 32]),
            timezone: "Asia/Kolkata".into(),
            last_login_time: None,
        };
        let key = vec![0u8; 64];
        let issuer = vec![1u8; 64];
        let token = jwt::issue_token(&user, &key, &issuer, Duration::hours(1)).unwrap();
        let claims = jwt::parse_token(&token, &key, &issuer).unwrap();
        assert_eq!(claims.sub, jwt::subject(&user));
        assert_eq!(jwt::user_id_from_subject(&claims.sub).unwrap(), 7);

        let mut changed = user.clone();
        changed.password_salt = Some(vec![8; 32]);
        assert_ne!(claims.sub, jwt::subject(&changed));
    }

    #[tokio::test]
    async fn record_login_sets_last_login_time() {
        use sea_orm::{ConnectionTrait, Database, DatabaseBackend, EntityTrait, Schema};

        use crate::{
            auth::{self, CreateUser},
            entities::user::Entity as UserEntity,
        };

        let db = Database::connect("sqlite::memory:").await.expect("sqlite");
        let schema = Schema::new(DatabaseBackend::Sqlite);
        let stmt = schema.create_table_from_entity(UserEntity);
        db.execute(&stmt).await.expect("create users");

        let user = auth::create_user(
            &db,
            CreateUser {
                name: "Ada".into(),
                email: "ada@example.com".into(),
                phone: "1".into(),
                plain_password: "secret".into(),
                role: "unassigned".into(),
                timezone: None,
            },
        )
        .await
        .expect("create");
        assert!(user.last_login_time.is_none());

        auth::record_login(&db, user.id).await.expect("record");
        let reloaded = UserEntity::find_by_id(user.id)
            .one(&db)
            .await
            .unwrap()
            .expect("user");
        let logged_in_at = reloaded.last_login_time.expect("last login");
        assert!(logged_in_at <= chrono::Utc::now());
    }
}
