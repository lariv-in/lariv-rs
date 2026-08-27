//! Invoice preferences and payment preferences singletons.

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use crate::plugins::finance_accounts::{
    logic::journal::{credit_balance_type, debit_balance_type},
    validate_leaf_account_balance_type,
};
use crate::plugins::finance_products::preferences::optional_i64;

use crate::plugins::finance_invoices::entities::{
    payment_preferences::{self, Entity as PaymentPreferencesEntity},
    preferences::{self, Entity as InvoicePreferencesEntity},
};

pub async fn load_invoice_preferences(db: &DatabaseConnection) -> preferences::Model {
    if let Ok(Some(p)) = InvoicePreferencesEntity::find_by_id(1i64).one(db).await {
        return p;
    }
    let now = Utc::now();
    let default_format = "INV-{{YYYY}}-{{POSTED_SEQ}}".to_string();
    let am = preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        invoice_number_format: Set(Some(default_format.clone())),
        ..Default::default()
    };
    am.insert(db).await.unwrap_or(preferences::Model {
        id: 1,
        created_at: Some(now),
        updated_at: Some(now),
        account_receivable_id: None,
        account_revenue_id: None,
        account_tax_payable_id: None,
        journal_id: None,
        invoice_number_format: Some(default_format),
        invoice_pdf_template: None,
        invoice_logo_vnode_id: None,
        invoice_signature_vnode_id: None,
        company_name: None,
        company_address: None,
        company_phone: None,
        company_gstin: None,
        place_of_supply: None,
    })
}

pub async fn validate_invoice_preferences_for_posting(
    db: &DatabaseConnection,
    prefs: &preferences::Model,
) -> Result<(), String> {
    validate_leaf_account_balance_type(
        db,
        optional_i64(prefs.account_receivable_id),
        debit_balance_type(),
        "accounts receivable",
    )
    .await
    .map_err(|e| e.to_string())?;
    validate_leaf_account_balance_type(
        db,
        optional_i64(prefs.account_revenue_id),
        credit_balance_type(),
        "revenue account",
    )
    .await
    .map_err(|e| e.to_string())?;
    validate_leaf_account_balance_type(
        db,
        optional_i64(prefs.account_tax_payable_id),
        credit_balance_type(),
        "tax payable account",
    )
    .await
    .map_err(|e| e.to_string())?;
    if optional_i64(prefs.journal_id) == 0 {
        return Err("journal is required in invoice preferences".to_string());
    }
    Ok(())
}

pub async fn load_payment_preferences(db: &DatabaseConnection) -> payment_preferences::Model {
    if let Ok(Some(p)) = PaymentPreferencesEntity::find_by_id(1i64).one(db).await {
        return p;
    }
    let now = Utc::now();
    let am = payment_preferences::ActiveModel {
        id: Set(1),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };
    am.insert(db).await.unwrap_or(payment_preferences::Model {
        id: 1,
        created_at: Some(now),
        updated_at: Some(now),
        payment_account_id: None,
    })
}

pub async fn validate_payment_preferences_for_create(
    db: &DatabaseConnection,
    prefs: &payment_preferences::Model,
) -> Result<(), String> {
    let account_id = optional_i64(prefs.payment_account_id);
    if account_id == 0 {
        return Err("payment account is required in payment preferences".to_string());
    }
    validate_leaf_account_balance_type(db, account_id, debit_balance_type(), "payment account")
        .await
        .map_err(|e| e.to_string())
}
