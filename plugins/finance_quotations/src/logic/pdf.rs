//! Quotation PDF: Minijinja template to Typst.

use std::sync::Arc;

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde::Serialize;
use thiserror::Error;

use crate::entities::{
    quotation::{self, Entity as QuotationEntity},
    quotation_goods_line::{self, Entity as GoodsLineEntity},
    quotation_service_line::{self, Entity as ServiceLineEntity},
};
use crate::logic::editor::quotation_grand_total;
use crate::logic::prefs::load_preferences;
use crate::logic::tax::{load_goods_line_tax_ids, load_service_line_tax_ids};
use lariv_plugin_contacts::entities::{
    company::Entity as CompanyEntity, contact::Entity as ContactEntity,
};
use lariv_plugin_crm::entities::customer::Entity as CustomerEntity;
use lariv_plugin_filesystem::state::FilesystemState;
use lariv_plugin_finance_common::{decimal::decimal_display, typst};
use lariv_plugin_finance_invoices::VnodeImageContext;
use lariv_plugin_finance_invoices::logic::tax_calculations::invoice_line_amounts;
use lariv_plugin_finance_products::entities::product::Entity as ProductEntity;
use lariv_plugin_finance_services::entities::service::Entity as ServiceEntity;
use lariv_plugin_finance_taxes::{
    entities::tax::TaxKind,
    scope::{load_taxes_by_ids, tax_label},
};
use minijinja::{Environment, UndefinedBehavior};
use num2words::{Lang, Num2Words};

const DEFAULT_TEMPLATE: &str = include_str!("../templates/example_quotation_pdf.typ.tmpl");

#[derive(Debug, Error)]
pub enum PdfError {
    #[error("{0}")]
    Message(String),
    #[error("not found")]
    NotFound,
}

pub struct PdfResult {
    pub bytes: Vec<u8>,
    pub filename_base: String,
}

pub struct PdfParts {
    pub pdf: PdfResult,
    pub context: serde_json::Value,
}

#[derive(Serialize)]
struct PdfCustomer {
    #[serde(rename = "ID")]
    id: i64,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Address")]
    address: String,
    #[serde(rename = "GSTIN")]
    gstin: String,
    #[serde(rename = "PAN")]
    pan: String,
    #[serde(rename = "Phone")]
    phone: String,
    #[serde(rename = "Email")]
    email: String,
    #[serde(rename = "Website")]
    website: String,
}

#[derive(Serialize)]
struct PdfTax {
    #[serde(rename = "ID")]
    id: i64,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Percentage")]
    percentage: String,
    #[serde(rename = "TaxType")]
    tax_type: String,
}

#[derive(Serialize)]
struct PdfLine {
    #[serde(rename = "ID")]
    id: i64,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Details")]
    details: String,
    #[serde(rename = "Quantity")]
    quantity: String,
    #[serde(rename = "Rate")]
    rate: String,
    #[serde(rename = "PreTax")]
    pre_tax: String,
    #[serde(rename = "LineTotal")]
    line_total: String,
    #[serde(rename = "FinalCost")]
    final_cost: String,
    #[serde(rename = "Remarks")]
    remarks: String,
    #[serde(rename = "Taxes")]
    taxes: String,
    #[serde(rename = "TaxItems")]
    tax_items: Vec<PdfTax>,
}

#[derive(Serialize)]
struct PdfRoot {
    #[serde(rename = "ID")]
    id: i64,
    #[serde(rename = "InvoiceNumber")]
    invoice_number: String,
    #[serde(rename = "Number")]
    number: String,
    #[serde(rename = "CustomerId")]
    customer_id: i64,
    #[serde(rename = "Date")]
    date: String,
    #[serde(rename = "Customer")]
    customer: PdfCustomer,
    #[serde(rename = "GoodsLines")]
    goods_lines: Vec<PdfLine>,
    #[serde(rename = "ServiceLines")]
    service_lines: Vec<PdfLine>,
    #[serde(rename = "GrandTotal")]
    grand_total: String,
    #[serde(rename = "Remarks")]
    remarks: String,
    #[serde(rename = "BankAccount")]
    bank_account: String,
    company_name: String,
    company_address: String,
    company_phone: String,
    company_gstin: String,
    place_of_supply: String,
    company_logo_vnode_id: i64,
    company_signature_vnode_id: i64,
    preview: bool,
}

pub async fn render_quotation_pdf(
    db: &DatabaseConnection,
    fs: Option<&FilesystemState>,
    id: i64,
) -> Result<PdfResult, PdfError> {
    Ok(render_quotation_pdf_parts(db, fs, id).await?.pdf)
}

pub async fn render_quotation_pdf_parts(
    db: &DatabaseConnection,
    fs: Option<&FilesystemState>,
    id: i64,
) -> Result<PdfParts, PdfError> {
    let quotation = QuotationEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|err| PdfError::Message(err.to_string()))?
        .ok_or(PdfError::NotFound)?;
    let prefs = load_preferences(db).await;
    let goods = goods_pdf_lines(db, quotation.id).await?;
    let services = service_pdf_lines(db, quotation.id).await?;
    let grand = quotation_grand_total(db, quotation.id).await;
    let customer = load_customer(db, quotation.customer_id).await?;
    let root = PdfRoot {
        id: quotation.id,
        invoice_number: quotation.number.clone(),
        number: quotation.number.clone(),
        customer_id: quotation.customer_id,
        date: lariv_core::datetime::format_date(quotation.date),
        customer,
        goods_lines: goods,
        service_lines: services,
        grand_total: decimal_display(grand),
        remarks: quotation.remarks.clone(),
        bank_account: quotation.bank_account.clone().unwrap_or_default(),
        company_name: prefs.company_name.clone().unwrap_or_default(),
        company_address: prefs.company_address.clone().unwrap_or_default(),
        company_phone: prefs.company_phone.clone().unwrap_or_default(),
        company_gstin: prefs.company_gstin.clone().unwrap_or_default(),
        place_of_supply: prefs.place_of_supply.clone().unwrap_or_default(),
        company_logo_vnode_id: prefs.company_logo_vnode_id.unwrap_or(0),
        company_signature_vnode_id: prefs.company_signature_vnode_id.unwrap_or(0),
        preview: false,
    };
    let context = serde_json::to_value(&root)
        .map_err(|err| PdfError::Message(format!("serialize quotation PDF context: {err}")))?;
    let template = prefs
        .quotation_pdf_template
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or(DEFAULT_TEMPLATE);
    let bytes = compile_pdf(template, context.clone(), amount_words(grand), fs).await?;
    let filename_base = pdf_filename(&quotation);
    Ok(PdfParts {
        pdf: PdfResult {
            bytes,
            filename_base,
        },
        context,
    })
}

fn pdf_filename(quotation: &quotation::Model) -> String {
    let raw = quotation.number.trim();
    let base = if raw.is_empty() {
        format!("quotation-{}", quotation.id)
    } else {
        raw.to_string()
    };
    base.chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            other => other,
        })
        .collect()
}

async fn goods_pdf_lines(
    db: &DatabaseConnection,
    quotation_id: i64,
) -> Result<Vec<PdfLine>, PdfError> {
    let lines = GoodsLineEntity::find()
        .filter(quotation_goods_line::Column::QuotationId.eq(quotation_id))
        .order_by_asc(quotation_goods_line::Column::Id)
        .all(db)
        .await
        .map_err(|err| PdfError::Message(err.to_string()))?;
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let name = ProductEntity::find_by_id(line.product_id)
            .one(db)
            .await
            .ok()
            .flatten()
            .map(|row| row.name)
            .unwrap_or_else(|| format!("Product #{}", line.product_id));
        let tax_ids = load_goods_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        out.push(
            pdf_line(
                db,
                line.id,
                name,
                line.quantity,
                line.rate,
                line.pre_tax_amount,
                line.remarks.as_deref().unwrap_or(""),
                &tax_ids,
            )
            .await,
        );
    }
    Ok(out)
}

async fn service_pdf_lines(
    db: &DatabaseConnection,
    quotation_id: i64,
) -> Result<Vec<PdfLine>, PdfError> {
    let lines = ServiceLineEntity::find()
        .filter(quotation_service_line::Column::QuotationId.eq(quotation_id))
        .order_by_asc(quotation_service_line::Column::Id)
        .all(db)
        .await
        .map_err(|err| PdfError::Message(err.to_string()))?;
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let name = ServiceEntity::find_by_id(line.service_id)
            .one(db)
            .await
            .ok()
            .flatten()
            .map(|row| row.name)
            .unwrap_or_else(|| format!("Service #{}", line.service_id));
        let tax_ids = load_service_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        out.push(
            pdf_line(
                db,
                line.id,
                name,
                line.quantity,
                line.rate,
                line.pre_tax_amount,
                line.remarks.as_deref().unwrap_or(""),
                &tax_ids,
            )
            .await,
        );
    }
    Ok(out)
}

#[expect(clippy::too_many_arguments, reason = "PDF line fields stay explicit")]
async fn pdf_line(
    db: &DatabaseConnection,
    id: i64,
    name: String,
    quantity: Decimal,
    rate: Decimal,
    pre_tax: Decimal,
    remarks: &str,
    tax_ids: &[i64],
) -> PdfLine {
    let taxes = load_taxes_by_ids(db, tax_ids).await.unwrap_or_default();
    let (_, _, _, net) = invoice_line_amounts(pre_tax, &taxes);
    let tax_items = taxes
        .iter()
        .map(|tax| PdfTax {
            id: tax.id,
            name: tax_label(tax),
            percentage: decimal_display(tax.percentage),
            tax_type: match tax.tax_type {
                TaxKind::Withholding => "withholding",
                TaxKind::Levied => "levied",
            }
            .to_string(),
        })
        .collect();
    let labels = taxes.iter().map(tax_label).collect::<Vec<_>>().join(", ");
    PdfLine {
        id,
        name,
        details: format!("{} × {}", decimal_display(quantity), decimal_display(rate)),
        quantity: decimal_display(quantity),
        rate: decimal_display(rate),
        pre_tax: decimal_display(pre_tax),
        line_total: decimal_display(net),
        final_cost: decimal_display(net),
        remarks: remarks.to_string(),
        taxes: if labels.is_empty() {
            "—".to_string()
        } else {
            labels
        },
        tax_items,
    }
}

async fn load_customer(db: &DatabaseConnection, customer_id: i64) -> Result<PdfCustomer, PdfError> {
    let Some(customer) = CustomerEntity::find_by_id(customer_id)
        .one(db)
        .await
        .map_err(|err| PdfError::Message(err.to_string()))?
    else {
        return Ok(empty_customer(customer_id));
    };
    if customer.is_individual {
        let contact = match customer.individual_id {
            Some(id) => ContactEntity::find_by_id(id).one(db).await.ok().flatten(),
            None => None,
        };
        let company = match contact.as_ref().and_then(|row| row.company_id) {
            Some(id) => CompanyEntity::find_by_id(id).one(db).await.ok().flatten(),
            None => None,
        };
        Ok(PdfCustomer {
            id: customer_id,
            name: contact
                .as_ref()
                .map(|row| row.name.clone())
                .unwrap_or_default(),
            address: company.as_ref().map(company_address).unwrap_or_default(),
            gstin: company
                .as_ref()
                .and_then(|row| row.gstin.clone())
                .unwrap_or_default(),
            pan: company
                .as_ref()
                .and_then(|row| row.pan.clone())
                .unwrap_or_default(),
            phone: contact
                .as_ref()
                .and_then(|row| row.phone.clone())
                .unwrap_or_default(),
            email: contact
                .as_ref()
                .and_then(|row| row.email.clone())
                .unwrap_or_default(),
            website: company
                .as_ref()
                .and_then(|row| row.website.clone())
                .unwrap_or_default(),
        })
    } else {
        let company = match customer.company_id {
            Some(id) => CompanyEntity::find_by_id(id).one(db).await.ok().flatten(),
            None => None,
        };
        Ok(PdfCustomer {
            id: customer_id,
            name: company
                .as_ref()
                .map(|row| row.name.clone())
                .unwrap_or_default(),
            address: company.as_ref().map(company_address).unwrap_or_default(),
            gstin: company
                .as_ref()
                .and_then(|row| row.gstin.clone())
                .unwrap_or_default(),
            pan: company
                .as_ref()
                .and_then(|row| row.pan.clone())
                .unwrap_or_default(),
            phone: company
                .as_ref()
                .and_then(|row| row.phone.clone())
                .unwrap_or_default(),
            email: company
                .as_ref()
                .and_then(|row| row.email.clone())
                .unwrap_or_default(),
            website: company.and_then(|row| row.website).unwrap_or_default(),
        })
    }
}

fn empty_customer(id: i64) -> PdfCustomer {
    PdfCustomer {
        id,
        name: String::new(),
        address: String::new(),
        gstin: String::new(),
        pan: String::new(),
        phone: String::new(),
        email: String::new(),
        website: String::new(),
    }
}

fn company_address(company: &lariv_plugin_contacts::entities::company::Model) -> String {
    [
        company.address_line_1.as_deref(),
        company.address_line_2.as_deref(),
        company.city.as_deref(),
        company.state.as_deref(),
        company.pincode.as_deref(),
    ]
    .into_iter()
    .flatten()
    .filter(|part| !part.trim().is_empty())
    .collect::<Vec<_>>()
    .join(", ")
}

fn amount_words(amount: Decimal) -> String {
    let rounded = amount.round().to_string();
    let n = rounded.parse::<i64>().unwrap_or(0);
    let words = Num2Words::new(n)
        .lang(Lang::English)
        .to_words()
        .unwrap_or_default();
    format!("{words} Rupees")
}

async fn compile_pdf(
    template: &str,
    ctx: serde_json::Value,
    grand_words: String,
    fs: Option<&FilesystemState>,
) -> Result<Vec<u8>, PdfError> {
    if let Some(fs) = fs {
        let work_dir = typst::typst_work_dir();
        let vnode = VnodeImageContext::new(fs.db.clone(), Arc::clone(&fs.store), work_dir.clone());
        let typst_src = render_template(template, &ctx, &grand_words, Some(&vnode))?;
        let result = typst::typst_compile_in(&work_dir, &typst_src)
            .await
            .map_err(|err| PdfError::Message(format!("Typst compile failed:\n{err}")));
        if let Err(err) = std::fs::remove_dir_all(&work_dir) {
            tracing::warn!(error = %err, "failed to remove quotation pdf work dir");
        }
        result
    } else {
        let typst_src = render_template(template, &ctx, &grand_words, None)?;
        typst::typst_compile(&typst_src)
            .await
            .map_err(|err| PdfError::Message(format!("Typst compile failed:\n{err}")))
    }
}

fn render_template(
    template: &str,
    ctx: &serde_json::Value,
    grand_words: &str,
    vnode: Option<&VnodeImageContext>,
) -> Result<String, PdfError> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::SemiStrict);
    let words = grand_words.to_string();
    env.add_function("grandTotalWords", move || {
        Ok::<_, minijinja::Error>(words.clone())
    });
    env.add_function("num2words", |n: i64| {
        Ok::<_, minijinja::Error>(
            Num2Words::new(n)
                .lang(Lang::English)
                .to_words()
                .unwrap_or_default(),
        )
    });
    env.add_function("num2wordsAnd", |n: i64| {
        Ok::<_, minijinja::Error>(
            Num2Words::new(n)
                .lang(Lang::English)
                .to_words()
                .unwrap_or_default(),
        )
    });
    env.add_function("num2wordsRupees", |n: i64| {
        let words = Num2Words::new(n)
            .lang(Lang::English)
            .to_words()
            .unwrap_or_default();
        Ok::<_, minijinja::Error>(format!("{words} Rupees"))
    });
    if let Some(vnode) = vnode {
        let vnode = vnode.clone();
        env.add_function(
            "vnodeImage",
            move |vnode_id: i64| -> Result<String, minijinja::Error> {
                if vnode_id <= 0 {
                    return Err(minijinja::Error::new(
                        minijinja::ErrorKind::InvalidOperation,
                        "vnodeImage: no file selected",
                    ));
                }
                vnode.resolve_sync(vnode_id).map_err(|err| {
                    minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, err)
                })
            },
        );
    }
    env.add_template("quotation", template)
        .map_err(|err| PdfError::Message(format!("Email or PDF template: {err}")))?;
    env.get_template("quotation")
        .map_err(|err| PdfError::Message(format!("PDF template: {err}")))?
        .render(ctx)
        .map_err(|err| PdfError::Message(format!("PDF template: {err}")))
}
