#[cfg(test)]
mod integration {
    use lariv_rs::app::App;
    use lariv_rs::plugins::users::{self, UsersTag, auth, entities::user::Entity as UserEntity};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    #[tokio::test]
    async fn migrate_seed_and_authenticate() {
        let cfg_path = std::env::temp_dir().join(format!(
            "lariv-users-test-{}.toml",
            std::process::id()
        ));
        std::fs::write(
            &cfg_path,
            r#"
database_url = "sqlite::memory:"
[p_users]
adminEmail = "admin@test.local"
adminPassword = "supersecret"
"#,
        )
        .unwrap();

        let app = App::new_web_app();
        let app = users::install(app);
        let app = app.load_config(&cfg_path).await.expect("load_config");
        let _ = std::fs::remove_file(&cfg_path);
        let app = app.mount();
        app.run_migrations().await.expect("migrations");
        app.run_seeds().await.expect("seed");

        let state = app.get_capability_output::<UsersTag, _>();

        let admin = UserEntity::find()
            .filter(lariv_rs::plugins::users::entities::user::Column::Email.eq("admin@test.local"))
            .one(&state.db)
            .await
            .unwrap()
            .expect("admin exists");
        assert!(admin.is_superuser);

        let user = auth::authenticate(&state.db, "admin@test.local", "supersecret")
            .await
            .expect("auth");
        assert_eq!(user.id, admin.id);
    }
}
