//! Postgres integration test: create draft invoice via HTTP and verify DB state.

#![recursion_limit = "512"]

use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use lariv_rs::app::App;
use lariv_rs::db::DbTag;
use lariv_rs::http::into_axum_router;
use lariv_rs::plugins::contacts::entities::{company as company_entity, contact as contact_entity};
use lariv_rs::plugins::finance_invoices::entities::{
    DraftInvoiceEntity, DraftInvoiceGoodsLineEntity, DraftPaymentTermEntity,
    DraftPaymentTermLineEntity, draft_invoice_goods_line, draft_payment_term_line,
};
use lariv_rs::plugins::finance_invoices::logic::tax_assoc::load_draft_line_tax_ids;
use lariv_rs::plugins::finance_products::entities::product;
use lariv_rs::plugins::finance_products::preferences::set_product_tax_ids;
use lariv_rs::plugins::finance_taxes::entities::tax::{self, TaxKind};
use lariv_rs::plugins::users::{self, UsersTag, auth, entities::user::Entity as UserEntity};
use lariv_rs::plugins::{
    contacts, crm, customer, dashboard, filesystem, finance_accounts, finance_creditnotes,
    finance_indian, finance_invoices, finance_products, finance_services, finance_taxes, inventory,
    llm_assistant, otp, pwa,
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
    let app = contacts::install(app);
    let app = crm::install(app);
    let app = customer::install(app);
    let app = finance_creditnotes::install(app);
    let app = finance_taxes::install(app);
    let app = inventory::install(app);
    let app = finance_products::install(app);
    let app = finance_services::install(app);
    let app = finance_invoices::install(app);
    let app = finance_indian::install(app);
    let app = otp::install(app);
    let app = pwa::install(app);
    let app = dashboard::install(app);

    let app = app.load_config(&path).await.expect("load_config");
    std::fs::remove_file(&path).ok();
    let app = app.mount();
    app.run_migrations().await.expect("migrations");
    app.run_seeds().await.expect("seed");

    let db = app.get_capability_output::<DbTag, _>().conn.clone();
    let users_state = app.get_capability_output::<UsersTag, _>();

    let company = company_entity::ActiveModel {
        name: Set("Test Company".into()),
        gstin: Set(Some("27AAAAA0000A1Z5".into())),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("company");
    let contact = contact_entity::ActiveModel {
        name: Set("Test Person".into()),
        email: Set(Some("person@test.local".into())),
        is_primary: Set(false),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("contact");
    let company_customer_row = lariv_rs::plugins::crm::entities::customer::ActiveModel {
        is_individual: Set(false),
        company_id: Set(Some(company.id)),
        individual_id: Set(None),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("company customer");
    let individual_customer_row = lariv_rs::plugins::crm::entities::customer::ActiveModel {
        is_individual: Set(true),
        company_id: Set(None),
        individual_id: Set(Some(contact.id)),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("individual customer");

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

    let stock = lariv_rs::plugins::inventory::entities::stock::ActiveModel {
        name: Set("Main".into()),
        qty_type: Set("quantity".into()),
        qty_unit: Set(String::new()),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("stock");
    let prod = product::ActiveModel {
        name: Set("Test Product".into()),
        stock_id: Set(stock.id),
        reference: Set(Some("REF-001".into())),
        base_price_formula: Set(r#"decimal("40")"#.into()),
        sales_price_formula: Set(r#"decimal("100")"#.into()),
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
    let csrf = lariv_rs::html_form::generate_csrf_token();
    let body = format!(
        "csrf_token={csrf}&number=&datetime=2025-06-01T12:00&CustomerID={}&PaymentTermLinesJSON={}&InvoiceGoodsLinesJSON={}",
        company_customer_row.id,
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
                .header("cookie", format!("auth-token={token}; csrf_token={csrf}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let drafts = DraftInvoiceEntity::find().all(&db).await.expect("drafts");
    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].customer_id, company_customer_row.id);

    let individual_body = format!(
        "csrf_token={csrf}&number=&datetime=2025-06-02T12:00&CustomerID={}&PaymentTermLinesJSON={}&InvoiceGoodsLinesJSON={}",
        individual_customer_row.id,
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
                .header("cookie", format!("auth-token={token}; csrf_token={csrf}"))
                .body(Body::from(individual_body))
                .unwrap(),
        )
        .await
        .expect("individual response");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let individual = DraftInvoiceEntity::find()
        .filter(
            lariv_rs::plugins::finance_invoices::entities::draft_invoice::Column::Datetime
                .gt(drafts[0].datetime),
        )
        .one(&db)
        .await
        .expect("query individual")
        .expect("individual draft");
    assert_eq!(individual.customer_id, individual_customer_row.id);

    let term_id = drafts[0]
        .draft_payment_term_id
        .expect("draft payment term id");
    let term = DraftPaymentTermEntity::find_by_id(term_id)
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
    assert_eq!(
        pt_lines[0].amount_kind,
        lariv_rs::plugins::finance_invoices::PaymentTermAmountKind::Relative
    );
    assert_eq!(pt_lines[0].amount_percentage, Some(Decimal::from(100)));

    let lines = DraftInvoiceGoodsLineEntity::find()
        .filter(draft_invoice_goods_line::Column::DraftInvoiceId.eq(drafts[0].id))
        .all(&db)
        .await
        .expect("lines");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].product_id, prod.id);
    assert_eq!(lines[0].quantity, Decimal::from(2));
    assert_eq!(lines[0].rate, Decimal::from(50));
    assert_eq!(lines[0].pre_tax_amount, Decimal::from(100));

    let line_tax_ids = load_draft_line_tax_ids(&db, lines[0].id)
        .await
        .expect("line taxes");
    assert_eq!(line_tax_ids, vec![tax.id]);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Postgres DATABASE_URL"]
async fn create_draft_invoice_via_rune_env() {
    use lariv_rs::plugins::filesystem::FilesystemTag;
    use lariv_rs::plugins::llm_assistant::rune_engine;
    use lariv_rs::rune_env::{RuneEnvCtx, RuneEnvTag};
    use serde_json::json;

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration test");
    let path = temp_config(&database_url);

    let app = App::new_web_app();
    let app = users::install(app);
    let app = filesystem::install(app);
    let app = llm_assistant::install(app);
    let app = finance_accounts::install(app);
    let app = contacts::install(app);
    let app = crm::install(app);
    let app = customer::install(app);
    let app = finance_creditnotes::install(app);
    let app = finance_taxes::install(app);
    let app = inventory::install(app);
    let app = finance_products::install(app);
    let app = finance_services::install(app);
    let app = finance_invoices::install(app);
    let app = finance_indian::install(app);
    let app = otp::install(app);
    let app = pwa::install(app);
    let app = dashboard::install(app);

    let app = app.load_config(&path).await.expect("load_config");
    std::fs::remove_file(&path).ok();
    let app = app.mount();
    app.run_migrations().await.expect("migrations");
    app.run_seeds().await.expect("seed");

    let db = app.get_capability_output::<DbTag, _>().conn.clone();
    let rune_env = app.get_capability_output::<RuneEnvTag, _>();
    let store = app
        .get_capability_output::<FilesystemTag, _>()
        .store
        .clone();

    let contact = contact_entity::ActiveModel {
        name: Set("Rune Person".into()),
        is_primary: Set(false),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("contact");

    let customer = lariv_rs::plugins::crm::entities::customer::ActiveModel {
        is_individual: Set(true),
        company_id: Set(None),
        individual_id: Set(Some(contact.id)),
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

    let stock = lariv_rs::plugins::inventory::entities::stock::ActiveModel {
        name: Set("Main".into()),
        qty_type: Set("quantity".into()),
        qty_unit: Set(String::new()),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("stock");
    let prod = product::ActiveModel {
        name: Set("Rune Product".into()),
        stock_id: Set(stock.id),
        reference: Set(Some("RUNE-001".into())),
        base_price_formula: Set(r#"decimal("40")"#.into()),
        sales_price_formula: Set(r#"decimal("100")"#.into()),
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

    let number = format!("RUNE-{}", uuid::Uuid::new_v4());
    let env_ctx = RuneEnvCtx {
        db: &db,
        store,
        session_id: None,
    };
    let source = r#"
        create_invoice(#{
            number: number,
            customer_id: customer_id,
            datetime: "2025-06-01",
            lines: [#{ product_id: product_id, quantity: "2", rate: "50.0" }],
            payment_term_lines: [#{
                date_kind: "relative",
                due_duration: "15 days",
                amount_kind: "relative",
                amount_percentage: "100"
            }]
        })
    "#;
    let out = rune_engine::compile_and_run(
        rune_env,
        &env_ctx,
        source,
        &[
            ("number".into(), json!(number)),
            ("customer_id".into(), json!(customer.id)),
            ("product_id".into(), json!(prod.id)),
        ],
    )
    .await;
    assert!(
        out.get("error").is_none(),
        "rune create_invoice failed: {out}"
    );

    let draft = DraftInvoiceEntity::find()
        .filter(
            lariv_rs::plugins::finance_invoices::entities::draft_invoice::Column::Number
                .eq(number.clone()),
        )
        .one(&db)
        .await
        .expect("query draft")
        .expect("draft created by rune");
    assert_eq!(draft.customer_id, customer.id);
    assert_eq!(draft.number.as_deref(), Some(number.as_str()));

    let term_id = draft.draft_payment_term_id.expect("draft payment term id");
    let pt_lines = DraftPaymentTermLineEntity::find()
        .filter(draft_payment_term_line::Column::DraftPaymentTermId.eq(term_id))
        .all(&db)
        .await
        .expect("payment term lines");
    assert_eq!(pt_lines.len(), 1);
    let pt_line = pt_lines.first().expect("payment term line");
    assert_eq!(
        pt_line.amount_kind,
        lariv_rs::plugins::finance_invoices::PaymentTermAmountKind::Relative
    );
    assert_eq!(pt_line.amount_percentage, Some(Decimal::from(100)));

    let lines = DraftInvoiceGoodsLineEntity::find()
        .filter(draft_invoice_goods_line::Column::DraftInvoiceId.eq(draft.id))
        .all(&db)
        .await
        .expect("lines");
    assert_eq!(lines.len(), 1);
    let line = lines.first().expect("invoice line");
    assert_eq!(line.product_id, prod.id);
    assert_eq!(line.quantity, Decimal::from(2));
    assert_eq!(line.rate, Decimal::from(50));
    assert_eq!(line.pre_tax_amount, Decimal::from(100));

    let line_tax_ids = load_draft_line_tax_ids(&db, line.id)
        .await
        .expect("line taxes");
    assert_eq!(line_tax_ids, vec![tax.id]);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires Postgres DATABASE_URL"]
async fn post_invoice_uses_product_and_service_journal_templates() {
    use lariv_rs::plugins::finance_accounts::BalanceType;
    use lariv_rs::plugins::finance_accounts::entities::{
        account, currency, journal, journal_entry_item,
    };
    use lariv_rs::plugins::finance_invoices::entities::draft_invoice_service_line;
    use lariv_rs::plugins::finance_invoices::logic::{
        CreatePaymentInput, create_payment, draft_new_posted,
    };
    use lariv_rs::plugins::finance_invoices::payment_journal_template::save_payment_template;
    use lariv_rs::plugins::finance_products::journal_template::save_product_template;
    use lariv_rs::plugins::finance_services::entities::service;
    use sea_orm::QueryOrder;

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration test");
    let path = temp_config(&database_url);
    let app = App::new_web_app();
    let app = users::install(app);
    let app = filesystem::install(app);
    let app = llm_assistant::install(app);
    let app = finance_accounts::install(app);
    let app = contacts::install(app);
    let app = crm::install(app);
    let app = customer::install(app);
    let app = finance_creditnotes::install(app);
    let app = finance_taxes::install(app);
    let app = inventory::install(app);
    let app = finance_products::install(app);
    let app = finance_services::install(app);
    let app = finance_invoices::install(app);
    let app = finance_indian::install(app);
    let app = otp::install(app);
    let app = pwa::install(app);
    let app = dashboard::install(app);
    let app = app.load_config(&path).await.expect("load_config");
    std::fs::remove_file(&path).ok();
    let app = app.mount();
    app.run_migrations().await.expect("migrations");
    app.run_seeds().await.expect("seed");
    let db = app.get_capability_output::<DbTag, _>().conn.clone();

    let now = Utc::now();
    let currency = match currency::Entity::find().one(&db).await.expect("currencies") {
        Some(existing) => existing,
        None => currency::ActiveModel {
            code: Set(999),
            name: Set("Test Currency".into()),
            symbol: Set("T".into()),
            minor_unit: Set(2),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("currency"),
    };
    let journal = journal::ActiveModel {
        name: Set("Sales".into()),
        is_active: Set(true),
        is_mutable: Set(true),
        currency_id: Set(currency.id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("journal");
    let other_journal = journal::ActiveModel {
        name: Set("Other".into()),
        is_active: Set(true),
        is_mutable: Set(true),
        currency_id: Set(currency.id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("other journal");
    let receivable = account::ActiveModel {
        name: Set("Accounts receivable".into()),
        code: Set(9100),
        is_group: Set(false),
        balance_type: Set(BalanceType::Debit),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("receivable");
    let revenue = account::ActiveModel {
        name: Set("Revenue".into()),
        code: Set(9400),
        is_group: Set(false),
        balance_type: Set(BalanceType::Credit),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("revenue");
    let template_lines = serde_json::json!([
        {"account_id": receivable.id, "formula": "pre_tax"},
        {"account_id": revenue.id, "formula": "decimal(\"0\") - pre_tax"}
    ])
    .to_string();
    let schema = lariv_rs::plugins::finance_products::pricing::load_schema("{}").expect("schema");

    let stock = lariv_rs::plugins::inventory::entities::stock::ActiveModel {
        name: Set("Main".into()),
        qty_type: Set("quantity".into()),
        qty_unit: Set(String::new()),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("stock");
    let mut product_ids = Vec::new();
    for name in ["Widget", "Gadget"] {
        let prod = product::ActiveModel {
            name: Set(name.into()),
            stock_id: Set(stock.id),
            base_price_formula: Set(r#"decimal("40")"#.into()),
            sales_price_formula: Set(r#"decimal("100")"#.into()),
            hsn_code: Set(1),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("product");
        save_product_template(
            &db,
            prod.id,
            &journal.id.to_string(),
            &template_lines,
            &schema,
        )
        .await
        .expect("template");
        product_ids.push(prod.id);
    }
    let mismatched = product::ActiveModel {
        name: Set("Mismatch".into()),
        stock_id: Set(stock.id),
        base_price_formula: Set(r#"decimal("40")"#.into()),
        sales_price_formula: Set(r#"decimal("100")"#.into()),
        hsn_code: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("mismatched product");
    save_product_template(
        &db,
        mismatched.id,
        &other_journal.id.to_string(),
        &template_lines,
        &schema,
    )
    .await
    .expect("mismatched template");

    let tax = tax::ActiveModel {
        name: Set("GST 18%".into()),
        percentage: Set(Decimal::from(18)),
        tax_type: Set(TaxKind::Levied),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("tax");
    let tax_lines = serde_json::json!([
        {"account_id": receivable.id, "formula": "base * percentage / decimal(\"100\")"},
        {"account_id": revenue.id, "formula": "decimal(\"0\") - base * percentage / decimal(\"100\")"}
    ])
    .to_string();
    lariv_rs::plugins::finance_taxes::journal_template::save_tax_template(
        &db,
        tax.id,
        &journal.id.to_string(),
        &tax_lines,
    )
    .await
    .expect("tax template");
    set_product_tax_ids(&db, product_ids[0], &[tax.id])
        .await
        .expect("product tax");

    let company = company_entity::ActiveModel {
        name: Set("Buyer".into()),
        gstin: Set(Some("27AAAAA0000A1Z5".into())),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("company");
    let customer = lariv_rs::plugins::crm::entities::customer::ActiveModel {
        is_individual: Set(false),
        company_id: Set(Some(company.id)),
        individual_id: Set(None),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("customer");

    let admin = UserEntity::find()
        .filter(lariv_rs::plugins::users::entities::user::Column::Email.eq("admin@test.local"))
        .one(&db)
        .await
        .expect("query admin")
        .expect("admin user");
    let users_state = app.get_capability_output::<UsersTag, _>();
    let token =
        auth::login_token(&admin, &users_state.signing_key, &users_state.jwt_issuer).expect("jwt");
    let csrf = lariv_rs::html_form::generate_csrf_token();
    let payment_term_lines_json = urlencoding::encode(
        r#"[{"date_kind":"relative","due_date":"","due_duration":"15 days","amount_kind":"relative","amount":"","amount_percentage":"100"}]"#,
    );
    let lines_json = format!(
        r#"[{{"product_id":{},"quantity":"2","rate":"50"}},{{"product_id":{},"quantity":"1","rate":"40"}}]"#,
        product_ids[0], product_ids[1]
    );
    let body = format!(
        "csrf_token={csrf}&number=&datetime=2025-06-01T12:00&CustomerID={}&PaymentTermLinesJSON={}&InvoiceGoodsLinesJSON={}",
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
                .header("cookie", format!("auth-token={token}; csrf_token={csrf}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("create draft");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let draft = DraftInvoiceEntity::find()
        .one(&db)
        .await
        .expect("query draft")
        .expect("draft");

    let posted = draft_new_posted(&db, draft.id, Utc::now(), "UTC")
        .await
        .expect("post");
    assert_eq!(posted.journal_id, journal.id);
    let items = journal_entry_item::Entity::find()
        .filter(journal_entry_item::Column::JournalEntryId.eq(posted.journal_entry_id))
        .order_by_asc(journal_entry_item::Column::Id)
        .all(&db)
        .await
        .expect("journal items");
    assert_eq!(items.len(), 6);
    let receivable_sum: Decimal = items
        .iter()
        .filter(|item| item.account_id == receivable.id)
        .map(|item| item.amount)
        .sum();
    // Product pre-tax 140 plus 18% of the taxed line's 100.
    assert_eq!(receivable_sum, Decimal::from(158));
    assert!(
        items
            .iter()
            .all(|item| item.account_id == receivable.id || item.account_id == revenue.id)
    );

    let bank = account::ActiveModel {
        name: Set("Bank".into()),
        code: Set(1100),
        is_group: Set(false),
        balance_type: Set(BalanceType::Debit),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("bank");
    let payment_lines = serde_json::json!([
        {"account_id": bank.id, "formula": "amount - withholding"},
        {"account_id": receivable.id, "formula": "decimal(\"0\") - amount"}
    ])
    .to_string();
    lariv_rs::plugins::finance_invoices::logic::preferences::load_payment_preferences(&db).await;
    save_payment_template(&db, &journal.id.to_string(), &payment_lines)
        .await
        .expect("payment template");
    let over = create_payment(
        &db,
        CreatePaymentInput {
            posted_invoice_id: posted.id,
            amount: Decimal::from(159),
            datetime: Utc::now(),
            withholding_tax_ids: Vec::new(),
        },
    )
    .await;
    match over {
        Err(error) => assert!(error.contains("open balance"), "{error}"),
        Ok(_) => panic!("payment above the template receivable should be rejected"),
    }

    let service = service::ActiveModel {
        name: Set("Install".into()),
        sac_code: Set(1),
        variables: Set("{}".into()),
        base_price_formula: Set(r#"decimal("1")"#.into()),
        sales_price_formula: Set(r#"decimal("1")"#.into()),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("service");
    let reject_lines = format!(
        r#"[{{"product_id":{},"quantity":"1","rate":"10"}},{{"product_id":{},"quantity":"1","rate":"10"}}]"#,
        product_ids[0], mismatched.id
    );
    let reject_body = format!(
        "csrf_token={csrf}&number=&datetime=2025-06-03T12:00&CustomerID={}&PaymentTermLinesJSON={}&InvoiceGoodsLinesJSON={}",
        customer.id,
        payment_term_lines_json,
        urlencoding::encode(&reject_lines),
    );
    let router = into_axum_router(&app);
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/finance-invoices/create")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", format!("auth-token={token}; csrf_token={csrf}"))
                .body(Body::from(reject_body))
                .unwrap(),
        )
        .await
        .expect("mismatched draft");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let mismatched_draft = DraftInvoiceEntity::find()
        .filter(
            lariv_rs::plugins::finance_invoices::entities::draft_invoice::Column::Id.ne(draft.id),
        )
        .one(&db)
        .await
        .expect("query mismatched")
        .expect("mismatched draft");
    let mismatch_err = draft_new_posted(&db, mismatched_draft.id, Utc::now(), "UTC")
        .await
        .expect_err("mismatched journals");
    assert!(mismatch_err.contains("same journal"), "{mismatch_err}");
    let service_lines = format!(
        r#"[{{"product_id":{},"quantity":"1","rate":"10"}}]"#,
        product_ids[0]
    );
    let service_body = format!(
        "csrf_token={csrf}&number=&datetime=2025-06-04T12:00&CustomerID={}&PaymentTermLinesJSON={}&InvoiceGoodsLinesJSON={}",
        customer.id,
        payment_term_lines_json,
        urlencoding::encode(&service_lines),
    );
    let router = into_axum_router(&app);
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/finance-invoices/create")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", format!("auth-token={token}; csrf_token={csrf}"))
                .body(Body::from(service_body))
                .unwrap(),
        )
        .await
        .expect("service draft");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let service_draft = DraftInvoiceEntity::find()
        .order_by_desc(lariv_rs::plugins::finance_invoices::entities::draft_invoice::Column::Id)
        .one(&db)
        .await
        .expect("query service draft")
        .expect("service draft");
    draft_invoice_service_line::ActiveModel {
        draft_invoice_id: Set(service_draft.id),
        service_id: Set(service.id),
        rate: Set(Decimal::ONE),
        quantity: Set(Decimal::ONE),
        variable_values: Set("{}".into()),
        pre_tax_amount: Set(Decimal::ONE),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("service line");
    let service_err = draft_new_posted(&db, service_draft.id, Utc::now(), "UTC")
        .await
        .expect_err("service without a journal template");
    assert!(
        service_err.contains("no journal entry template"),
        "{service_err}"
    );
    let service_schema =
        lariv_rs::plugins::finance_services::pricing::load_schema("{}").expect("service schema");
    lariv_rs::plugins::finance_services::journal_template::save_service_template(
        &db,
        service.id,
        &journal.id.to_string(),
        &template_lines,
        &service_schema,
    )
    .await
    .expect("service template");
    let service_posted = draft_new_posted(&db, service_draft.id, Utc::now(), "UTC")
        .await
        .expect("post service draft");
    assert_eq!(service_posted.journal_id, journal.id);
    let posted_service_lines =
        lariv_rs::plugins::finance_invoices::entities::posted_invoice_service_line::Entity::find()
            .filter(
                lariv_rs::plugins::finance_invoices::entities::posted_invoice_service_line::Column::PostedInvoiceId
                    .eq(service_posted.id),
            )
            .all(&db)
            .await
            .expect("posted service lines");
    assert_eq!(posted_service_lines.len(), 1);
    let service_item = journal_entry_item::Entity::find_by_id(
        posted_service_lines
            .first()
            .expect("posted service line")
            .journal_entry_item_id,
    )
    .one(&db)
    .await
    .expect("query service journal item")
    .expect("service journal item");
    assert_eq!(service_item.account_id, receivable.id);
    assert_eq!(service_item.amount, Decimal::ONE);

    let other_tax = tax::ActiveModel {
        name: Set("Other GST".into()),
        percentage: Set(Decimal::from(18)),
        tax_type: Set(TaxKind::Levied),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("other tax");
    lariv_rs::plugins::finance_taxes::journal_template::save_tax_template(
        &db,
        other_tax.id,
        &other_journal.id.to_string(),
        &tax_lines,
    )
    .await
    .expect("other tax template");
    let tax_body = format!(
        "csrf_token={csrf}&number=&datetime=2025-06-05T12:00&CustomerID={}&PaymentTermLinesJSON={}&InvoiceGoodsLinesJSON={}",
        customer.id,
        payment_term_lines_json,
        urlencoding::encode(&format!(
            r#"[{{"product_id":{},"quantity":"1","rate":"10"}}]"#,
            product_ids[1]
        )),
    );
    let router = into_axum_router(&app);
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/finance-invoices/create")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", format!("auth-token={token}; csrf_token={csrf}"))
                .body(Body::from(tax_body))
                .unwrap(),
        )
        .await
        .expect("tax journal draft");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let tax_draft = DraftInvoiceEntity::find()
        .order_by_desc(lariv_rs::plugins::finance_invoices::entities::draft_invoice::Column::Id)
        .one(&db)
        .await
        .expect("query tax draft")
        .expect("tax draft");
    lariv_rs::plugins::finance_invoices::logic::tax_assoc::set_draft_invoice_taxes(
        &db,
        tax_draft.id,
        &[other_tax.id],
    )
    .await
    .expect("header tax");
    let tax_err = draft_new_posted(&db, tax_draft.id, Utc::now(), "UTC")
        .await
        .expect_err("tax journal");
    assert!(tax_err.contains("same journal"), "{tax_err}");
}
