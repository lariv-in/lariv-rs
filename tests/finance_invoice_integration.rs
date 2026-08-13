//! Postgres integration test: create draft invoice via HTTP and verify DB state.

#![recursion_limit = "512"]

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use lariv_rs::app::App;
use lariv_rs::db::DbTag;
use lariv_rs::http::into_axum_router;
use lariv_rs::plugins::customer::entities::customer as customer_entity;
use lariv_rs::plugins::finance_invoices::entities::{
    DraftInvoiceEntity, DraftInvoiceLineEntity, DraftPaymentTermEntity, DraftPaymentTermLineEntity,
    draft_invoice_line, draft_payment_term, draft_payment_term_line,
};
use lariv_rs::plugins::finance_invoices::logic::tax_assoc::load_draft_line_tax_ids;
use lariv_rs::plugins::finance_products::entities::product;
use lariv_rs::plugins::finance_products::preferences::set_product_tax_ids;
use lariv_rs::plugins::finance_taxes::entities::tax::{self, TaxKind};
use lariv_rs::plugins::users::{self, UsersTag, auth, entities::user::Entity as UserEntity};
use lariv_rs::plugins::{
    customer, dashboard, filesystem, finance_accounts, finance_creditnotes, finance_customer,
    finance_fiscal_year, finance_indian, finance_invoices, finance_products, finance_taxes,
    llm_assistant, no_signup, otp, pwa,
};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use tower::ServiceExt;

fn temp_config(database_url: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lariv-invoice-it-{}-{}.toml",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let body = format!(
        r#"
database_url = "{database_url}"
[users]
adminEmail = "admin@test.local"
adminPassword = "adminadmin"
signingKey = "dGVzdC1zaWduaW5nLWtleS1wYWRkZWQtdG8tNjQtYnl0ZXMhISEhISEhISEhISE="
jwtIssuer = "dW5pcXVpdHktdGVzdC1pc3N1ZXItcGFkZGVkLXRvLTY0LWJ5dGVzIQ=="
"#
    );
    std::fs::write(&path, body).expect("write temp config");
    path
}

#[tokio::test]
#[ignore = "requires Postgres DATABASE_URL"]
async fn create_draft_invoice_via_http() {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration test");
    let path = temp_config(&database_url);

    let app = App::new_web_app();
    let app = users::install(app);
    let app = filesystem::install(app);
    let app = llm_assistant::install(app);
    let app = finance_accounts::install(app);
    let app = customer::install(app);
    let app = finance_customer::install(app);
    let app = finance_creditnotes::install(app);
    let app = finance_fiscal_year::install(app);
    let app = finance_taxes::install(app);
    let app = finance_products::install(app);
    let app = finance_invoices::install(app);
    let app = finance_indian::install(app);
    let app = otp::install(app);
    let app = no_signup::install(app);
    let app = pwa::install(app);
    let app = dashboard::install(app);

    let app = app.load_config(&path).await.expect("load_config");
    std::fs::remove_file(&path).ok();
    let app = app.mount();
    app.run_migrations().await.expect("migrations");
    app.run_seeds().await.expect("seed");

    let db = app.get_capability_output::<DbTag, _>().conn.clone();
    let users_state = app.get_capability_output::<UsersTag, _>();

    let customer = customer_entity::ActiveModel {
        name: Set("Test Customer".into()),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("customer");

    let tax = tax::ActiveModel {
        name: Set("GST 18%".into()),
        percentage: Set(Decimal::from(18)),
        tax_type: Set(TaxKind::Levied),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("tax");

    let prod = product::ActiveModel {
        name: Set("Test Product".into()),
        product_type: Set(product::ProductType::Goods),
        reference: Set(Some("REF-001".into())),
        base_cost: Set(Decimal::from(40)),
        sales_price: Set(Decimal::from(100)),
        hsn_code: Set(1234),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("product");
    set_product_tax_ids(&db, prod.id, &[tax.id])
        .await
        .expect("product taxes");

    let admin = UserEntity::find()
        .filter(lariv_rs::plugins::users::entities::user::Column::Email.eq("admin@test.local"))
        .one(&db)
        .await
        .expect("query admin")
        .expect("admin user");
    let token =
        auth::login_token(&admin, &users_state.signing_key, &users_state.jwt_issuer).expect("jwt");

    let lines_json = format!(
        r#"[{{"product_id":{},"quantity":"2","rate":"50.0"}}]"#,
        prod.id
    );
    let payment_term_lines_json = urlencoding::encode(
        r#"[{"date_kind":"relative","due_date":"","due_duration":"15 days","amount_kind":"relative","amount":"","amount_percentage":"100"}]"#,
    );
    let body = format!(
        "number=&datetime=2025-06-01T12:00&CustomerID={}&PaymentTermLinesJSON={}&InvoiceLinesJSON={}",
        customer.id,
        payment_term_lines_json,
        urlencoding::encode(&lines_json),
    );

    let router = into_axum_router(&app);
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/finance-invoices/create")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", format!("auth-token={token}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let drafts = DraftInvoiceEntity::find().all(&db).await.expect("drafts");
    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].customer_id, customer.id);

    let term = DraftPaymentTermEntity::find()
        .filter(draft_payment_term::Column::DraftInvoiceId.eq(drafts[0].id))
        .one(&db)
        .await
        .expect("payment term")
        .expect("draft payment term");

    let pt_lines = DraftPaymentTermLineEntity::find()
        .filter(draft_payment_term_line::Column::DraftPaymentTermId.eq(term.id))
        .all(&db)
        .await
        .expect("payment term lines");
    assert_eq!(pt_lines.len(), 1);
    assert_eq!(pt_lines[0].amount_kind, "relative");
    assert_eq!(pt_lines[0].amount_percentage, Some(Decimal::from(100)));

    let lines = DraftInvoiceLineEntity::find()
        .filter(draft_invoice_line::Column::DraftInvoiceId.eq(drafts[0].id))
        .all(&db)
        .await
        .expect("lines");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].product_id, prod.id);
    assert_eq!(lines[0].quantity, Decimal::from(2));
    assert_eq!(lines[0].rate, Decimal::from(50));

    let line_tax_ids = load_draft_line_tax_ids(&db, lines[0].id)
        .await
        .expect("line taxes");
    assert_eq!(line_tax_ids, vec![tax.id]);
}
