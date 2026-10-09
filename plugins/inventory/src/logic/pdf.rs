//! Stock movement PDF: Minijinja template → Typst → PDF.

use std::sync::Arc;

use lariv_plugin_contacts::entities::{
    company::{self, Entity as CompanyEntity},
    contact::Entity as ContactEntity,
};
use lariv_plugin_filesystem::state::FilesystemState;
use minijinja::{Environment, UndefinedBehavior};
use sea_orm::EntityTrait;
use serde::Serialize;

use crate::entities::preferences::Model as Preferences;
use crate::entities::stock_movement::Model as Movement;
use crate::logic::party::BillTo;
use crate::logic::pdf_assets::VnodeImageContext;
use crate::logic::preferences::load_preferences;
use crate::movement_type::MovementType;
use lariv_core::typst;

pub const DEFAULT_MOVEMENT_IN_TEMPLATE: &str = include_str!("../templates/movement_in.typ.tmpl");
pub const DEFAULT_MOVEMENT_OUT_TEMPLATE: &str = include_str!("../templates/movement_out.typ.tmpl");

#[derive(Debug)]
pub enum MovementPdfError {
    NotFound,
    Message(String),
}

impl MovementPdfError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}

pub struct MovementPdfResult {
    pub bytes: Vec<u8>,
    pub filename_base: String,
}

#[derive(Serialize)]
struct PdfParty {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Address")]
    address: String,
    #[serde(rename = "Phone")]
    phone: String,
    #[serde(rename = "Email")]
    email: String,
    #[serde(rename = "Gstin")]
    gstin: String,
}

#[derive(Serialize)]
struct PdfLine {
    #[serde(rename = "Stock")]
    stock: String,
    #[serde(rename = "Qty")]
    qty: String,
    #[serde(rename = "Unit")]
    unit: String,
    #[serde(rename = "QtyType")]
    qty_type: String,
}

#[derive(Serialize)]
struct PdfRoot {
    #[serde(rename = "Number")]
    number: String,
    #[serde(rename = "Datetime")]
    datetime: String,
    #[serde(rename = "MovementType")]
    movement_type: String,
    #[serde(rename = "Customer")]
    customer: PdfParty,
    #[serde(rename = "VehicleType")]
    vehicle_type: String,
    #[serde(rename = "VehicleNumber")]
    vehicle_number: String,
    #[serde(rename = "EwayBill")]
    eway_bill: String,
    #[serde(rename = "Driver")]
    driver: PdfParty,
    #[serde(rename = "Lines")]
    lines: Vec<PdfLine>,
    #[serde(rename = "CompanyName")]
    company_name: String,
    #[serde(rename = "CompanyAddress")]
    company_address: String,
    #[serde(rename = "CompanyPhone")]
    company_phone: String,
    #[serde(rename = "CompanyEmail")]
    company_email: String,
    #[serde(rename = "CompanyGstin")]
    company_gstin: String,
    #[serde(rename = "Terms")]
    terms: String,
    #[serde(rename = "Logo")]
    logo: i64,
    #[serde(rename = "Signature")]
    signature: i64,
}

pub struct PdfLineInput {
    pub stock: String,
    pub qty: String,
    pub unit: String,
    pub qty_type: String,
}

pub struct MovementPdfInput<'a> {
    pub number: &'a str,
    pub datetime_label: &'a str,
    pub movement_type: MovementType,
    pub bill_to: BillTo,
    pub vehicle_type: &'a str,
    pub vehicle_number: &'a str,
    pub eway_bill: &'a str,
    pub driver_id: Option<i64>,
    pub lines: Vec<PdfLineInput>,
    pub template_override: Option<&'a str>,
    pub company_name: Option<&'a str>,
    pub company_address: Option<&'a str>,
    pub company_phone: Option<&'a str>,
    pub company_email: Option<&'a str>,
    pub company_gstin: Option<&'a str>,
    pub terms: Option<&'a str>,
    pub logo_vnode_id: Option<i64>,
    pub signature_vnode_id: Option<i64>,
    /// Preview uses sample party text instead of looking up a contact or company.
    pub sample: bool,
}

fn opt_text(value: &Option<String>) -> String {
    value.clone().unwrap_or_default()
}

fn default_template(kind: MovementType) -> String {
    match kind {
        MovementType::In => DEFAULT_MOVEMENT_IN_TEMPLATE.to_string(),
        MovementType::Out => DEFAULT_MOVEMENT_OUT_TEMPLATE.to_string(),
    }
}

/// `override_src = None` uses the saved template, then the built-in challan.
/// `Some` is the unsaved preferences field: blank skips the saved template.
fn template_for(prefs: &Preferences, kind: MovementType, override_src: Option<&str>) -> String {
    if let Some(src) = override_src {
        let src = src.trim();
        if src.is_empty() {
            return default_template(kind);
        }
        return src.to_string();
    }
    let saved = match kind {
        MovementType::In => prefs.movement_in_template.as_deref(),
        MovementType::Out => prefs.movement_out_template.as_deref(),
    };
    saved
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| default_template(kind))
}

async fn customer_party(db: &sea_orm::DatabaseConnection, bill: BillTo) -> PdfParty {
    if bill.bill_to_individual {
        let id = bill.customer_individual.unwrap_or(0);
        let contact = ContactEntity::find_by_id(id).one(db).await.ok().flatten();
        PdfParty {
            name: contact
                .as_ref()
                .map(|row| row.name.clone())
                .unwrap_or_else(|| {
                    if id > 0 {
                        format!("#{id}")
                    } else {
                        String::new()
                    }
                }),
            address: String::new(),
            phone: contact
                .as_ref()
                .and_then(|row| row.phone.clone())
                .unwrap_or_default(),
            email: contact
                .as_ref()
                .and_then(|row| row.email.clone())
                .unwrap_or_default(),
            gstin: String::new(),
        }
    } else {
        let id = bill.customer_company.unwrap_or(0);
        let company = CompanyEntity::find_by_id(id).one(db).await.ok().flatten();
        PdfParty {
            name: company
                .as_ref()
                .map(|row| row.name.clone())
                .unwrap_or_else(|| {
                    if id > 0 {
                        format!("#{id}")
                    } else {
                        String::new()
                    }
                }),
            address: company
                .as_ref()
                .and_then(company::Model::formatted_address_for_typst)
                .unwrap_or_default(),
            phone: company
                .as_ref()
                .and_then(|row| row.phone.clone())
                .unwrap_or_default(),
            email: company
                .as_ref()
                .and_then(|row| row.email.clone())
                .unwrap_or_default(),
            gstin: company
                .as_ref()
                .and_then(|row| row.gstin.clone())
                .unwrap_or_default(),
        }
    }
}

fn empty_party() -> PdfParty {
    PdfParty {
        name: String::new(),
        address: String::new(),
        phone: String::new(),
        email: String::new(),
        gstin: String::new(),
    }
}

async fn driver_party(db: &sea_orm::DatabaseConnection, id: Option<i64>) -> PdfParty {
    let Some(id) = id.filter(|id| *id > 0) else {
        return empty_party();
    };
    let contact = ContactEntity::find_by_id(id).one(db).await.ok().flatten();
    PdfParty {
        name: contact
            .as_ref()
            .map(|row| row.name.clone())
            .unwrap_or_else(|| format!("#{id}")),
        address: String::new(),
        phone: contact
            .as_ref()
            .and_then(|row| row.phone.clone())
            .unwrap_or_default(),
        email: contact.and_then(|row| row.email).unwrap_or_default(),
        gstin: String::new(),
    }
}

fn apply_company(root: &mut PdfRoot, prefs: &Preferences, input: &MovementPdfInput<'_>) {
    root.company_name = input
        .company_name
        .map(str::to_string)
        .unwrap_or_else(|| opt_text(&prefs.company_name));
    root.company_address = input
        .company_address
        .map(str::to_string)
        .unwrap_or_else(|| opt_text(&prefs.company_address));
    root.company_phone = input
        .company_phone
        .map(str::to_string)
        .unwrap_or_else(|| opt_text(&prefs.company_phone));
    root.company_email = input
        .company_email
        .map(str::to_string)
        .unwrap_or_else(|| opt_text(&prefs.company_email));
    root.company_gstin = input
        .company_gstin
        .map(str::to_string)
        .unwrap_or_else(|| opt_text(&prefs.company_gstin));
    root.terms = input
        .terms
        .map(str::to_string)
        .unwrap_or_else(|| opt_text(&prefs.terms_and_conditions));
    root.logo = input.logo_vnode_id.or(prefs.logo_vnode_id).unwrap_or(0);
    root.signature = input
        .signature_vnode_id
        .or(prefs.signature_vnode_id)
        .unwrap_or(0);
}

/// Escape characters that Typst treats specially in content mode.
fn typst_plain(value: String) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' | '@' | '#' | '[' | ']' | '*' | '_' | '`' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

fn render_template(
    tmpl_src: &str,
    root: &PdfRoot,
    vnode_ctx: &VnodeImageContext,
) -> Result<String, MovementPdfError> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::SemiStrict);
    env.add_function("typst", typst_plain);
    let vnode_ctx = vnode_ctx.clone();
    env.add_function(
        "vnodeImage",
        move |vnode_id: i64| -> Result<String, minijinja::Error> {
            vnode_ctx
                .resolve_sync(vnode_id)
                .map_err(|err| minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, err))
        },
    );
    let template = env.template_from_str(tmpl_src).map_err(|err| {
        MovementPdfError::msg(format!("invalid stock movement PDF template: {err}"))
    })?;
    let value = minijinja::Value::from_serialize(root);
    template.render(value).map_err(|err| {
        MovementPdfError::msg(format!(
            "rendering stock movement PDF template failed: {err}"
        ))
    })
}

pub async fn render_movement_pdf(
    fs: &FilesystemState,
    input: MovementPdfInput<'_>,
) -> Result<MovementPdfResult, MovementPdfError> {
    let prefs = load_preferences(&fs.db)
        .await
        .map_err(MovementPdfError::msg)?;
    let tmpl = template_for(&prefs, input.movement_type, input.template_override);
    let (customer, driver) = if input.sample {
        (
            PdfParty {
                name: "Sample customer".into(),
                address: "12 Market Road \\ Pune".into(),
                phone: "020-40000000".into(),
                email: "accounts@example.com".into(),
                gstin: "27AAAAA0000A1Z5".into(),
            },
            PdfParty {
                name: "Sample driver".into(),
                address: String::new(),
                phone: "9000000000".into(),
                email: String::new(),
                gstin: String::new(),
            },
        )
    } else {
        (
            customer_party(&fs.db, input.bill_to).await,
            driver_party(&fs.db, input.driver_id).await,
        )
    };
    let mut root = PdfRoot {
        number: input.number.to_string(),
        datetime: input.datetime_label.to_string(),
        movement_type: input.movement_type.label().to_string(),
        customer,
        vehicle_type: input.vehicle_type.to_string(),
        vehicle_number: input.vehicle_number.to_string(),
        eway_bill: input.eway_bill.to_string(),
        driver,
        lines: input
            .lines
            .iter()
            .map(|line| PdfLine {
                stock: line.stock.clone(),
                qty: line.qty.clone(),
                unit: line.unit.clone(),
                qty_type: line.qty_type.clone(),
            })
            .collect(),
        company_name: String::new(),
        company_address: String::new(),
        company_phone: String::new(),
        company_email: String::new(),
        company_gstin: String::new(),
        terms: String::new(),
        logo: 0,
        signature: 0,
    };
    apply_company(&mut root, &prefs, &input);
    let work_dir = typst::typst_work_dir();
    std::fs::create_dir_all(&work_dir)
        .map_err(|err| MovementPdfError::msg(format!("create typst work dir: {err}")))?;
    let vnode_ctx = VnodeImageContext::new(fs.db.clone(), Arc::clone(&fs.store), work_dir.clone());
    let typst_src = render_template(&tmpl, &root, &vnode_ctx)?;
    let pdf_bytes = typst::typst_compile_in(&work_dir, &typst_src)
        .await
        .map_err(|err| MovementPdfError::msg(format!("Typst compile failed:\n{err}")))?;
    if let Err(err) = std::fs::remove_dir_all(&work_dir) {
        tracing::warn!(error = %err, path = %work_dir.display(), "failed to remove movement pdf work dir");
    }
    let filename_base = if input.number.trim().is_empty() {
        "stock-movement".to_string()
    } else {
        input
            .number
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    ch
                } else {
                    '-'
                }
            })
            .collect()
    };
    Ok(MovementPdfResult {
        bytes: pdf_bytes,
        filename_base,
    })
}

pub fn movement_pdf_input<'a>(
    movement: &'a Movement,
    datetime_label: &'a str,
    lines: Vec<PdfLineInput>,
) -> MovementPdfInput<'a> {
    MovementPdfInput {
        number: &movement.number,
        datetime_label,
        movement_type: movement.movement_type,
        bill_to: BillTo::from_row(
            movement.bill_to_individual,
            movement.customer_individual,
            movement.customer_company,
        ),
        vehicle_type: movement.vehicle_type.as_deref().unwrap_or(""),
        vehicle_number: movement.vehicle_number.as_deref().unwrap_or(""),
        eway_bill: movement.eway_bill.as_deref().unwrap_or(""),
        driver_id: movement.driver_id,
        lines,
        template_override: None,
        company_name: None,
        company_address: None,
        company_phone: None,
        company_email: None,
        company_gstin: None,
        terms: None,
        logo_vnode_id: None,
        signature_vnode_id: None,
        sample: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_templates_compile() {
        for src in [DEFAULT_MOVEMENT_IN_TEMPLATE, DEFAULT_MOVEMENT_OUT_TEMPLATE] {
            let mut env = Environment::new();
            env.set_undefined_behavior(UndefinedBehavior::SemiStrict);
            env.add_function("typst", typst_plain);
            let template = env.template_from_str(src).expect("template parses");
            let root = serde_json::json!({
                "Number": "SM-2026-1",
                "Datetime": "09/10/2026 12:00:00",
                "MovementType": "In",
                "Customer": {"Name": "Sample", "Address": "12 Market Road \\ Pune", "Phone": "", "Email": "", "Gstin": ""},
                "VehicleType": "Truck",
                "VehicleNumber": "MH12AB1234",
                "EwayBill": "391234567890",
                "Driver": {"Name": "Driver", "Address": "", "Phone": "", "Email": "", "Gstin": ""},
                "Lines": [{"Stock": "Cement", "Qty": "10", "Unit": "bag", "QtyType": "Count"}],
                "CompanyName": "Acme",
                "CompanyAddress": "Pune",
                "CompanyPhone": "020",
                "CompanyEmail": "a@b.c",
                "CompanyGstin": "27AAAAA0000A1Z5",
                "Terms": "Goods received in good condition.",
                "Logo": 0,
                "Signature": 0
            });
            let rendered = template
                .render(minijinja::Value::from_serialize(&root))
                .expect("render");
            typst::typst_compile(&rendered)
                .await
                .expect("typst compile");
        }
    }
}

pub fn sample_lines() -> Vec<PdfLineInput> {
    vec![
        PdfLineInput {
            stock: "Cement".into(),
            qty: "10".into(),
            unit: "bag".into(),
            qty_type: "Count".into(),
        },
        PdfLineInput {
            stock: "Sand".into(),
            qty: "2".into(),
            unit: "t".into(),
            qty_type: "Weight".into(),
        },
    ]
}
