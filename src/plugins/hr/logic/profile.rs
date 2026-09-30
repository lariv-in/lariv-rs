use chrono::NaiveDate;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::datetime::{format_date, parse_date};
use crate::plugins::documents::document_type::DocumentType;
use crate::plugins::documents::entities::document::{self, Entity as DocumentEntity};
use crate::plugins::documents::logic::load_type_summaries;
use crate::plugins::filesystem::entities::filesystem_node::Entity as VNodeEntity;
use crate::plugins::hr::blood_group::BloodGroup;
use crate::plugins::hr::entities::employee::{self, ActiveModel as EmployeeActive};
use crate::plugins::hr::forms::{ACCOUNT_TYPE_CHOICES, EmployeeForm, MARITAL_STATUS_CHOICES};
use crate::plugins::hr::gender::ApplicantGender;
use crate::plugins::hr::logic::applicant::parse_optional_fk;
use sea_orm::ActiveValue::Set;

#[derive(Clone, Debug)]
pub struct EmployeeProfile {
    pub fathers_name: String,
    pub date_of_birth: Option<NaiveDate>,
    pub gender: Option<ApplicantGender>,
    pub marital_status: String,
    pub nationality: String,
    pub is_disabled: bool,
    pub disability_type: String,
    pub photograph_vnode_id: Option<i64>,
    pub blood_group: Option<BloodGroup>,
    pub identification_mark: String,
    pub present_address: String,
    pub present_pin_code: String,
    pub permanent_address: String,
    pub permanent_pin_code: String,
    pub emergency_contact_name: String,
    pub emergency_contact_relation: String,
    pub emergency_contact_mobile: String,
    pub aadhar_document_id: Option<i64>,
    pub pan_document_id: Option<i64>,
    pub passport_document_id: Option<i64>,
    pub account_holder_name: String,
    pub account_number: String,
    pub account_ifsc_code: String,
    pub account_type: String,
    pub qualifications: String,
    pub date_of_joining: Option<NaiveDate>,
    pub probation_end_date: Option<NaiveDate>,
}

impl EmployeeProfile {
    pub fn empty() -> Self {
        Self {
            fathers_name: String::new(),
            date_of_birth: None,
            gender: None,
            marital_status: String::new(),
            nationality: String::new(),
            is_disabled: false,
            disability_type: String::new(),
            photograph_vnode_id: None,
            blood_group: None,
            identification_mark: String::new(),
            present_address: String::new(),
            present_pin_code: String::new(),
            permanent_address: String::new(),
            permanent_pin_code: String::new(),
            emergency_contact_name: String::new(),
            emergency_contact_relation: String::new(),
            emergency_contact_mobile: String::new(),
            aadhar_document_id: None,
            pan_document_id: None,
            passport_document_id: None,
            account_holder_name: String::new(),
            account_number: String::new(),
            account_ifsc_code: String::new(),
            account_type: String::new(),
            qualifications: String::new(),
            date_of_joining: None,
            probation_end_date: None,
        }
    }
}

pub fn apply_profile(am: &mut EmployeeActive, profile: &EmployeeProfile) {
    am.fathers_name = Set(profile.fathers_name.clone());
    am.date_of_birth = Set(profile.date_of_birth);
    am.gender = Set(profile.gender);
    am.marital_status = Set(profile.marital_status.clone());
    am.nationality = Set(profile.nationality.clone());
    am.is_disabled = Set(profile.is_disabled);
    am.disability_type = Set(profile.disability_type.clone());
    am.photograph_vnode_id = Set(profile.photograph_vnode_id);
    am.blood_group = Set(profile.blood_group);
    am.identification_mark = Set(profile.identification_mark.clone());
    am.present_address = Set(profile.present_address.clone());
    am.present_pin_code = Set(profile.present_pin_code.clone());
    am.permanent_address = Set(profile.permanent_address.clone());
    am.permanent_pin_code = Set(profile.permanent_pin_code.clone());
    am.emergency_contact_name = Set(profile.emergency_contact_name.clone());
    am.emergency_contact_relation = Set(profile.emergency_contact_relation.clone());
    am.emergency_contact_mobile = Set(profile.emergency_contact_mobile.clone());
    am.aadhar_document_id = Set(profile.aadhar_document_id);
    am.pan_document_id = Set(profile.pan_document_id);
    am.passport_document_id = Set(profile.passport_document_id);
    am.account_holder_name = Set(profile.account_holder_name.clone());
    am.account_number = Set(profile.account_number.clone());
    am.account_ifsc_code = Set(profile.account_ifsc_code.clone());
    am.account_type = Set(profile.account_type.clone());
    am.qualifications = Set(profile.qualifications.clone());
    am.date_of_joining = Set(profile.date_of_joining);
    am.probation_end_date = Set(profile.probation_end_date);
}

pub async fn profile_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &EmployeeForm,
) -> Result<EmployeeProfile, String> {
    let gender = if form.gender.trim().is_empty() {
        None
    } else {
        Some(ApplicantGender::parse(&form.gender).ok_or_else(|| "Choose a gender".to_string())?)
    };
    let marital_status = choice_or_empty(&form.marital_status, MARITAL_STATUS_CHOICES, "marital status")?;
    let blood_group = if form.blood_group.trim().is_empty() {
        None
    } else {
        Some(BloodGroup::parse(&form.blood_group).ok_or_else(|| "Choose a blood group".to_string())?)
    };
    let account_type = choice_or_empty(&form.account_type, ACCOUNT_TYPE_CHOICES, "account type")?;
    let photograph_vnode_id = parse_optional_fk(&form.photograph_vnode_id);
    if let Some(vnode_id) = photograph_vnode_id {
        require_file(db, vnode_id).await?;
    }
    let aadhar_document_id = require_document(
        db,
        parse_optional_fk(&form.aadhar_document_id),
        DocumentType::AadharCard,
        "Aadhar",
    )
    .await?;
    let pan_document_id = require_document(
        db,
        parse_optional_fk(&form.pan_document_id),
        DocumentType::Pan,
        "PAN",
    )
    .await?;
    let passport_document_id = require_document(
        db,
        parse_optional_fk(&form.passport_document_id),
        DocumentType::Passport,
        "Passport",
    )
    .await?;
    Ok(EmployeeProfile {
        fathers_name: form.fathers_name.trim().to_string(),
        date_of_birth: optional_date(&form.date_of_birth, "Date of birth")?,
        gender,
        marital_status,
        nationality: form.nationality.trim().to_string(),
        is_disabled: form.is_disabled,
        disability_type: form.disability_type.trim().to_string(),
        photograph_vnode_id,
        blood_group,
        identification_mark: form.identification_mark.trim().to_string(),
        present_address: form.present_address.trim().to_string(),
        present_pin_code: form.present_pin_code.trim().to_string(),
        permanent_address: form.permanent_address.trim().to_string(),
        permanent_pin_code: form.permanent_pin_code.trim().to_string(),
        emergency_contact_name: form.emergency_contact_name.trim().to_string(),
        emergency_contact_relation: form.emergency_contact_relation.trim().to_string(),
        emergency_contact_mobile: form.emergency_contact_mobile.trim().to_string(),
        aadhar_document_id,
        pan_document_id,
        passport_document_id,
        account_holder_name: form.account_holder_name.trim().to_string(),
        account_number: form.account_number.trim().to_string(),
        account_ifsc_code: form.account_ifsc_code.trim().to_string(),
        account_type,
        qualifications: form.qualifications.trim().to_string(),
        date_of_joining: optional_date(&form.date_of_joining, "Date of joining")?,
        probation_end_date: optional_date(&form.probation_end_date, "Probation end date")?,
    })
}

fn optional_date(raw: &str, label: &str) -> Result<Option<NaiveDate>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        Ok(None)
    } else {
        parse_date(raw)
            .map(Some)
            .ok_or_else(|| format!("{label} must be DD/MM/YYYY"))
    }
}

fn choice_or_empty(
    raw: &str,
    choices: &[(&str, &str)],
    label: &str,
) -> Result<String, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(String::new());
    }
    if choices.iter().any(|(key, _)| *key == raw) {
        Ok(raw.to_string())
    } else {
        Err(format!("Choose a {label}"))
    }
}

async fn require_file(db: &sea_orm::DatabaseConnection, vnode_id: i64) -> Result<(), String> {
    match VNodeEntity::find_by_id(vnode_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(node) if node.is_directory => Err("Photograph must be a file, not a folder".into()),
        Some(_) => Ok(()),
        None => Err("Photograph was not found".into()),
    }
}

async fn require_document(
    db: &sea_orm::DatabaseConnection,
    id: Option<i64>,
    expected: DocumentType,
    label: &str,
) -> Result<Option<i64>, String> {
    let Some(id) = id else {
        return Ok(None);
    };
    let doc = DocumentEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("{label} document was not found"))?;
    if doc.document_type != expected {
        return Err(format!("{label} must be a {} document", expected.label()));
    }
    Ok(Some(id))
}

#[derive(Clone, Debug, Default)]
pub struct EmployeeProfileView {
    pub fathers_name: String,
    pub date_of_birth: String,
    pub gender: String,
    pub marital_status: String,
    pub nationality: String,
    pub is_disabled: String,
    pub disability_type: String,
    pub photograph_href: String,
    pub photograph_name: String,
    pub blood_group: String,
    pub identification_mark: String,
    pub present_address: String,
    pub present_pin_code: String,
    pub permanent_address: String,
    pub permanent_pin_code: String,
    pub emergency_contact_name: String,
    pub emergency_contact_relation: String,
    pub emergency_contact_mobile: String,
    pub aadhar_href: String,
    pub aadhar_label: String,
    pub pan_href: String,
    pub pan_label: String,
    pub passport_href: String,
    pub passport_label: String,
    pub account_holder_name: String,
    pub account_number: String,
    pub account_ifsc_code: String,
    pub account_type: String,
    pub qualifications: String,
    pub date_of_joining: String,
    pub probation_end_date: String,
}

pub async fn profile_view(
    db: &sea_orm::DatabaseConnection,
    employee: &employee::Model,
) -> EmployeeProfileView {
    let (photograph_href, photograph_name) = vnode_view(db, employee.photograph_vnode_id).await;
    let (aadhar_href, aadhar_label) =
        document_view(db, employee.aadhar_document_id, DocumentType::AadharCard).await;
    let (pan_href, pan_label) = document_view(db, employee.pan_document_id, DocumentType::Pan).await;
    let (passport_href, passport_label) =
        document_view(db, employee.passport_document_id, DocumentType::Passport).await;
    EmployeeProfileView {
        fathers_name: employee.fathers_name.clone(),
        date_of_birth: employee.date_of_birth.map(format_date).unwrap_or_default(),
        gender: employee
            .gender
            .map(|gender| gender.label().to_string())
            .unwrap_or_default(),
        marital_status: choice_label(MARITAL_STATUS_CHOICES, &employee.marital_status),
        nationality: employee.nationality.clone(),
        is_disabled: if employee.is_disabled { "Yes" } else { "No" }.to_string(),
        disability_type: employee.disability_type.clone(),
        photograph_href,
        photograph_name,
        blood_group: employee
            .blood_group
            .map(|group| group.label().to_string())
            .unwrap_or_default(),
        identification_mark: employee.identification_mark.clone(),
        present_address: employee.present_address.clone(),
        present_pin_code: employee.present_pin_code.clone(),
        permanent_address: employee.permanent_address.clone(),
        permanent_pin_code: employee.permanent_pin_code.clone(),
        emergency_contact_name: employee.emergency_contact_name.clone(),
        emergency_contact_relation: employee.emergency_contact_relation.clone(),
        emergency_contact_mobile: employee.emergency_contact_mobile.clone(),
        aadhar_href,
        aadhar_label,
        pan_href,
        pan_label,
        passport_href,
        passport_label,
        account_holder_name: employee.account_holder_name.clone(),
        account_number: employee.account_number.clone(),
        account_ifsc_code: employee.account_ifsc_code.clone(),
        account_type: choice_label(ACCOUNT_TYPE_CHOICES, &employee.account_type),
        qualifications: employee.qualifications.clone(),
        date_of_joining: employee.date_of_joining.map(format_date).unwrap_or_default(),
        probation_end_date: employee
            .probation_end_date
            .map(format_date)
            .unwrap_or_default(),
    }
}

fn choice_label(choices: &[(&str, &str)], value: &str) -> String {
    choices
        .iter()
        .find(|(key, _)| *key == value)
        .map(|(_, label)| (*label).to_string())
        .unwrap_or_else(|| value.to_string())
}

pub(crate) async fn vnode_view(db: &sea_orm::DatabaseConnection, id: Option<i64>) -> (String, String) {
    let Some(id) = id.filter(|id| *id > 0) else {
        return (String::new(), String::new());
    };
    let name = crate::web::opt_or_log(VNodeEntity::find_by_id(id).one(db).await, "employee photograph")
        .map(|node| node.name)
        .unwrap_or_else(|| format!("File #{id}"));
    (
        crate::plugins::filesystem::routes::VNodeDetailRouteTag::new(id).url(),
        name,
    )
}

pub(crate) async fn document_view(
    db: &sea_orm::DatabaseConnection,
    id: Option<i64>,
    expected: DocumentType,
) -> (String, String) {
    let Some(id) = id.filter(|id| *id > 0) else {
        return (String::new(), String::new());
    };
    let Some(doc) = crate::web::opt_or_log(
        DocumentEntity::find_by_id(id)
            .filter(document::Column::DocumentType.eq(expected))
            .one(db)
            .await,
        "employee document",
    ) else {
        return (String::new(), format!("{expected} #{id}"));
    };
    let summaries = load_type_summaries(db, &[doc.clone()]).await.ok();
    let label = summaries
        .as_ref()
        .and_then(|rows| rows.get(&doc.id))
        .map(|summary| {
            if summary.number.is_empty() {
                summary.name.clone()
            } else if summary.name.is_empty() {
                summary.number.clone()
            } else {
                format!("{} · {}", summary.name, summary.number)
            }
        })
        .filter(|label| !label.is_empty())
        .unwrap_or_else(|| expected.label().to_string());
    (
        crate::plugins::documents::routes::DocumentDetailRouteTag::new(id).url(),
        label,
    )
}
