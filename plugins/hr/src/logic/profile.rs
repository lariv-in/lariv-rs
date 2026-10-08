use chrono::{NaiveDate, NaiveTime};
use rust_decimal::Decimal;
use sea_orm::ActiveValue::Set;
use sea_orm::EntityTrait;
use std::str::FromStr;

use crate::blood_group::BloodGroup;
use crate::entities::employee::{self, ActiveModel as EmployeeActive};
use crate::forms::{ACCOUNT_TYPE_CHOICES, EmployeeForm, MARITAL_STATUS_CHOICES};
use crate::gender::ApplicantGender;
use lariv_core::datetime::{format_date, format_time, parse_date, parse_time};
use lariv_core::html_form::{HtmlForm, UploadedFile};
use lariv_plugin_filesystem::entities::filesystem_node::Entity as VNodeEntity;
use lariv_plugin_filesystem::node::{self, NodeFile};
use lariv_plugin_filesystem::state::FilesystemState;

pub const HR_EMPLOYEES_DIR: &str = "HR Employees";

#[derive(Clone, Debug)]
pub struct EmployeeProfile {
    pub fathers_name: Option<String>,
    pub date_of_birth: Option<NaiveDate>,
    pub gender: Option<ApplicantGender>,
    pub marital_status: Option<String>,
    pub nationality: Option<String>,
    pub is_disabled: bool,
    pub disability_type: Option<String>,
    pub photograph_vnode_id: Option<i64>,
    pub blood_group: Option<BloodGroup>,
    pub identification_mark: Option<String>,
    pub present_address: Option<String>,
    pub present_pin_code: Option<String>,
    pub permanent_address: Option<String>,
    pub permanent_pin_code: Option<String>,
    pub emergency_contact_name: Option<String>,
    pub emergency_contact_relation: Option<String>,
    pub emergency_contact_mobile: Option<String>,
    pub aadhar_vnode_id: Option<i64>,
    pub pan_vnode_id: Option<i64>,
    pub passport_vnode_id: Option<i64>,
    pub account_holder_name: Option<String>,
    pub account_number: Option<String>,
    pub account_ifsc_code: Option<String>,
    pub account_type: Option<String>,
    pub qualifications: Option<String>,
    pub verified: bool,
    pub date_of_joining: Option<NaiveDate>,
    pub probation_end_date: Option<NaiveDate>,
    pub work_start: Option<NaiveTime>,
    pub work_end: Option<NaiveTime>,
    pub base_salary: Option<Decimal>,
    pub hourly_wage: Option<Decimal>,
    pub manager_id: Option<i64>,
}

impl EmployeeProfile {
    pub fn empty() -> Self {
        Self {
            fathers_name: None,
            date_of_birth: None,
            gender: None,
            marital_status: None,
            nationality: None,
            is_disabled: false,
            disability_type: None,
            photograph_vnode_id: None,
            blood_group: None,
            identification_mark: None,
            present_address: None,
            present_pin_code: None,
            permanent_address: None,
            permanent_pin_code: None,
            emergency_contact_name: None,
            emergency_contact_relation: None,
            emergency_contact_mobile: None,
            aadhar_vnode_id: None,
            pan_vnode_id: None,
            passport_vnode_id: None,
            account_holder_name: None,
            account_number: None,
            account_ifsc_code: None,
            account_type: None,
            qualifications: None,
            verified: false,
            date_of_joining: None,
            probation_end_date: None,
            work_start: None,
            work_end: None,
            base_salary: None,
            hourly_wage: None,
            manager_id: None,
        }
    }
}

pub fn apply_profile(am: &mut EmployeeActive, profile: &EmployeeProfile) {
    am.fathers_name = Set(profile.fathers_name.clone());
    am.date_of_birth = Set(profile.date_of_birth);
    am.gender = Set(profile.gender);
    am.marital_status = Set(profile.marital_status.clone());
    am.nationality = Set(profile.nationality.clone());
    am.is_disabled = Set(Some(profile.is_disabled));
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
    am.aadhar_vnode_id = Set(profile.aadhar_vnode_id);
    am.pan_vnode_id = Set(profile.pan_vnode_id);
    am.passport_vnode_id = Set(profile.passport_vnode_id);
    am.account_holder_name = Set(profile.account_holder_name.clone());
    am.account_number = Set(profile.account_number.clone());
    am.account_ifsc_code = Set(profile.account_ifsc_code.clone());
    am.account_type = Set(profile.account_type.clone());
    am.qualifications = Set(profile.qualifications.clone());
    am.verified = Set(profile.verified);
    am.date_of_joining = Set(profile.date_of_joining);
    am.probation_end_date = Set(profile.probation_end_date);
    am.work_start = Set(profile.work_start);
    am.work_end = Set(profile.work_end);
    am.base_salary = Set(profile.base_salary);
    am.hourly_wage = Set(profile.hourly_wage);
    am.manager_id = Set(profile.manager_id);
}

/// Labels of profile fields that are still empty.
///
/// Passport stays optional. Disability type is required only when the person is disabled.
/// Date of joining, probation end date, work hours, base salary, hourly wage, and manager
/// are set by an admin, so they do not block the employee from finishing this form.
pub fn employee_profile_gaps(employee: &employee::Model) -> Vec<&'static str> {
    let mut gaps = Vec::new();
    if text_missing(&employee.name) {
        gaps.push("name");
    }
    if text_missing(&employee.mobile) {
        gaps.push("mobile");
    }
    if text_missing(&employee.email) {
        gaps.push("email");
    }
    if text_missing(&employee.fathers_name) {
        gaps.push("father's name");
    }
    if employee.date_of_birth.is_none() {
        gaps.push("date of birth");
    }
    if employee.gender.is_none() {
        gaps.push("gender");
    }
    if text_missing(&employee.marital_status) {
        gaps.push("marital status");
    }
    if text_missing(&employee.nationality) {
        gaps.push("nationality");
    }
    if employee.is_disabled.is_none() {
        gaps.push("disability status");
    }
    if employee.is_disabled == Some(true) && text_missing(&employee.disability_type) {
        gaps.push("disability type");
    }
    if employee.photograph_vnode_id.is_none() {
        gaps.push("photograph");
    }
    if employee.blood_group.is_none() {
        gaps.push("blood group");
    }
    if text_missing(&employee.identification_mark) {
        gaps.push("identification mark");
    }
    if text_missing(&employee.present_address) {
        gaps.push("present address");
    }
    if text_missing(&employee.present_pin_code) {
        gaps.push("present PIN code");
    }
    if text_missing(&employee.permanent_address) {
        gaps.push("permanent address");
    }
    if text_missing(&employee.permanent_pin_code) {
        gaps.push("permanent PIN code");
    }
    if text_missing(&employee.emergency_contact_name) {
        gaps.push("emergency contact name");
    }
    if text_missing(&employee.emergency_contact_relation) {
        gaps.push("emergency contact relation");
    }
    if text_missing(&employee.emergency_contact_mobile) {
        gaps.push("emergency contact mobile");
    }
    if employee.aadhar_vnode_id.is_none() {
        gaps.push("Aadhar");
    }
    if employee.pan_vnode_id.is_none() {
        gaps.push("PAN");
    }
    if text_missing(&employee.account_holder_name) {
        gaps.push("bank account holder name");
    }
    if text_missing(&employee.account_number) {
        gaps.push("bank account number");
    }
    if text_missing(&employee.account_ifsc_code) {
        gaps.push("bank account IFSC code");
    }
    if text_missing(&employee.account_type) {
        gaps.push("bank account type");
    }
    if text_missing(&employee.qualifications) {
        gaps.push("qualifications");
    }
    gaps
}

pub fn employee_profile_complete(employee: &employee::Model) -> bool {
    employee_profile_gaps(employee).is_empty()
}

fn text_missing(value: &Option<String>) -> bool {
    value.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true)
}

pub async fn store_employee_file(
    fs: &FilesystemState,
    owner_id: i64,
    employee_name: &str,
    category: &str,
    file: UploadedFile,
) -> Result<i64, String> {
    let parent_id = node::ensure_directory_path(
        &fs.db,
        fs.store.as_ref(),
        None,
        &[HR_EMPLOYEES_DIR.to_string()],
    )
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "failed to create HR Employees folder".to_string())?;
    let parent = node::get_by_id(&fs.db, parent_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "HR Employees folder not found".to_string())?;

    let original = node::sanitize_node_name(file.filename());
    let stem = if original.is_empty() {
        category.to_string()
    } else {
        format!("{category}-{original}")
    };
    let prefix = node::sanitize_node_name(employee_name);
    let mut name = if prefix.is_empty() {
        stem.clone()
    } else {
        format!("{prefix}-{stem}")
    };
    let mut n = 2u32;
    while node::find_child(&fs.db, Some(parent.id), &name, false)
        .await
        .map_err(|e| e.to_string())?
        .is_some()
    {
        name = if prefix.is_empty() {
            format!("{stem}-{n}")
        } else {
            format!("{prefix}-{stem}-{n}")
        };
        n += 1;
        if n > 1000 {
            return Err("could not store employee file with a unique name".to_string());
        }
    }

    let vnode = node::create_owned(
        &fs.db,
        fs.store.as_ref(),
        name,
        false,
        Some(NodeFile::Upload(file)),
        Some(&parent),
        owner_id,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(vnode.id)
}

pub async fn profile_from_submit(
    fs: &FilesystemState,
    owner_id: i64,
    employee_name: &str,
    mut submit: <EmployeeForm as HtmlForm>::Submit,
    existing: Option<&employee::Model>,
) -> Result<EmployeeProfile, String> {
    let parsed = parse_profile_fields(
        &submit.gender,
        &submit.marital_status,
        &submit.blood_group,
        &submit.nationality,
        submit.is_disabled,
        &submit.disability_type,
        &submit.account_type,
        submit.same_as_present,
        &submit.present_address,
        &submit.present_pin_code,
        &submit.permanent_address,
        &submit.permanent_pin_code,
        &submit.fathers_name,
        &submit.date_of_birth,
        &submit.identification_mark,
        &submit.emergency_contact_name,
        &submit.emergency_contact_relation,
        &submit.emergency_contact_mobile,
        &submit.account_holder_name,
        &submit.account_number,
        &submit.account_ifsc_code,
        &submit.qualifications,
        &submit.date_of_joining,
        &submit.probation_end_date,
        &submit.work_start,
        &submit.work_end,
        &submit.base_salary,
        &submit.hourly_wage,
    )?;
    let manager_id = submit.manager_id.filter(|id| *id > 0);

    let photograph_vnode_id = match submit.photograph.take() {
        Some(file) => {
            Some(store_employee_file(fs, owner_id, employee_name, "photograph", file).await?)
        }
        None => existing.and_then(|e| e.photograph_vnode_id),
    };
    let aadhar_vnode_id = match submit.aadhar.take() {
        Some(file) => Some(store_employee_file(fs, owner_id, employee_name, "aadhar", file).await?),
        None => existing.and_then(|e| e.aadhar_vnode_id),
    };
    let pan_vnode_id = match submit.pan.take() {
        Some(file) => Some(store_employee_file(fs, owner_id, employee_name, "pan", file).await?),
        None => existing.and_then(|e| e.pan_vnode_id),
    };
    let passport_vnode_id = match submit.passport.take() {
        Some(file) => {
            Some(store_employee_file(fs, owner_id, employee_name, "passport", file).await?)
        }
        None => existing.and_then(|e| e.passport_vnode_id),
    };

    Ok(EmployeeProfile {
        photograph_vnode_id,
        aadhar_vnode_id,
        pan_vnode_id,
        passport_vnode_id,
        manager_id,
        verified: submit.verified,
        ..parsed
    })
}

pub async fn profile_from_form(
    _db: &sea_orm::DatabaseConnection,
    form: &EmployeeForm,
) -> Result<EmployeeProfile, String> {
    parse_profile_fields(
        &form.gender,
        &form.marital_status,
        &form.blood_group,
        &form.nationality,
        form.is_disabled,
        &form.disability_type,
        &form.account_type,
        form.same_as_present,
        &form.present_address,
        &form.present_pin_code,
        &form.permanent_address,
        &form.permanent_pin_code,
        &form.fathers_name,
        &form.date_of_birth,
        &form.identification_mark,
        &form.emergency_contact_name,
        &form.emergency_contact_relation,
        &form.emergency_contact_mobile,
        &form.account_holder_name,
        &form.account_number,
        &form.account_ifsc_code,
        &form.qualifications,
        &form.date_of_joining,
        &form.probation_end_date,
        &form.work_start,
        &form.work_end,
        &form.base_salary,
        &form.hourly_wage,
    )
    .map(|mut profile| {
        profile.manager_id = form.manager_id.filter(|id| *id > 0);
        profile.verified = form.verified;
        profile
    })
}

#[allow(clippy::too_many_arguments)]
fn parse_profile_fields(
    gender: &str,
    marital_status: &str,
    blood_group: &str,
    nationality: &str,
    is_disabled: bool,
    disability_type: &str,
    account_type: &str,
    same_as_present: bool,
    present_address: &str,
    present_pin_code: &str,
    permanent_address: &str,
    permanent_pin_code: &str,
    fathers_name: &str,
    date_of_birth: &str,
    identification_mark: &str,
    emergency_contact_name: &str,
    emergency_contact_relation: &str,
    emergency_contact_mobile: &str,
    account_holder_name: &str,
    account_number: &str,
    account_ifsc_code: &str,
    qualifications: &str,
    date_of_joining: &str,
    probation_end_date: &str,
    work_start: &str,
    work_end: &str,
    base_salary: &str,
    hourly_wage: &str,
) -> Result<EmployeeProfile, String> {
    let gender = if gender.trim().is_empty() {
        None
    } else {
        Some(ApplicantGender::parse(gender).ok_or_else(|| "Choose a gender".to_string())?)
    };
    let marital_status = optional_choice(marital_status, MARITAL_STATUS_CHOICES, "marital status")?;
    let blood_group = if blood_group.trim().is_empty() {
        None
    } else {
        Some(BloodGroup::parse(blood_group).ok_or_else(|| "Choose a blood group".to_string())?)
    };
    let nationality = optional_choice(nationality, crate::countries::ALL_COUNTRIES, "country")?;
    let disability_type = if is_disabled {
        optional_choice(
            disability_type,
            crate::disability::REGISTERED_DISABILITIES_INDIA,
            "disability type",
        )?
    } else {
        None
    };
    let account_type = optional_choice(account_type, ACCOUNT_TYPE_CHOICES, "account type")?;
    let present_address = optional_text(present_address);
    let present_pin_code = optional_text(present_pin_code);
    let (permanent_address, permanent_pin_code) = if same_as_present {
        (present_address.clone(), present_pin_code.clone())
    } else {
        (
            optional_text(permanent_address),
            optional_text(permanent_pin_code),
        )
    };

    Ok(EmployeeProfile {
        fathers_name: optional_text(fathers_name),
        date_of_birth: optional_date(date_of_birth, "Date of birth")?,
        gender,
        marital_status,
        nationality,
        is_disabled,
        disability_type,
        photograph_vnode_id: None,
        blood_group,
        identification_mark: optional_text(identification_mark),
        present_address,
        present_pin_code,
        permanent_address,
        permanent_pin_code,
        emergency_contact_name: optional_text(emergency_contact_name),
        emergency_contact_relation: optional_text(emergency_contact_relation),
        emergency_contact_mobile: optional_text(emergency_contact_mobile),
        aadhar_vnode_id: None,
        pan_vnode_id: None,
        passport_vnode_id: None,
        account_holder_name: optional_text(account_holder_name),
        account_number: optional_text(account_number),
        account_ifsc_code: optional_text(account_ifsc_code),
        account_type,
        qualifications: optional_text(qualifications),
        verified: false,
        date_of_joining: optional_date(date_of_joining, "Date of joining")?,
        probation_end_date: optional_date(probation_end_date, "Probation end date")?,
        work_start: optional_time(work_start, "Work start")?,
        work_end: optional_time(work_end, "Work end")?,
        base_salary: optional_money(base_salary, "Base salary")?,
        hourly_wage: optional_money(hourly_wage, "Hourly wage")?,
        manager_id: None,
    })
}

fn optional_text(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        None
    } else {
        Some(raw.to_string())
    }
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

fn optional_time(raw: &str, label: &str) -> Result<Option<NaiveTime>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        Ok(None)
    } else {
        parse_time(raw)
            .map(Some)
            .ok_or_else(|| format!("{label} must be HH:MM"))
    }
}

fn optional_money(raw: &str, label: &str) -> Result<Option<Decimal>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    let value = Decimal::from_str(raw).map_err(|_| format!("{label} must be a number"))?;
    if value.is_sign_negative() {
        return Err(format!("{label} cannot be negative"));
    }
    Ok(Some(value))
}

fn optional_choice(
    raw: &str,
    choices: &[(&str, &str)],
    label: &str,
) -> Result<Option<String>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    if choices.iter().any(|(key, _)| *key == raw) {
        Ok(Some(raw.to_string()))
    } else {
        Err(format!("Choose a {label}"))
    }
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
    pub verified: String,
    pub date_of_joining: String,
    pub probation_end_date: String,
    pub work_start: String,
    pub work_end: String,
    pub base_salary: String,
    pub hourly_wage: String,
    pub manager: String,
}

pub async fn profile_view(
    db: &sea_orm::DatabaseConnection,
    employee: &employee::Model,
) -> EmployeeProfileView {
    let (photograph_href, photograph_name) = vnode_view(db, employee.photograph_vnode_id).await;
    let (aadhar_href, aadhar_label) = vnode_view(db, employee.aadhar_vnode_id).await;
    let (pan_href, pan_label) = vnode_view(db, employee.pan_vnode_id).await;
    let (passport_href, passport_label) = vnode_view(db, employee.passport_vnode_id).await;
    EmployeeProfileView {
        fathers_name: show_text(&employee.fathers_name),
        date_of_birth: employee.date_of_birth.map(format_date).unwrap_or_default(),
        gender: employee
            .gender
            .map(|gender| gender.label().to_string())
            .unwrap_or_default(),
        marital_status: choice_label(MARITAL_STATUS_CHOICES, employee.marital_status.as_deref()),
        nationality: show_text(&employee.nationality),
        is_disabled: match employee.is_disabled {
            Some(true) => "Yes".to_string(),
            Some(false) => "No".to_string(),
            None => String::new(),
        },
        disability_type: show_text(&employee.disability_type),
        photograph_href,
        photograph_name,
        blood_group: employee
            .blood_group
            .map(|group| group.label().to_string())
            .unwrap_or_default(),
        identification_mark: show_text(&employee.identification_mark),
        present_address: show_text(&employee.present_address),
        present_pin_code: show_text(&employee.present_pin_code),
        permanent_address: show_text(&employee.permanent_address),
        permanent_pin_code: show_text(&employee.permanent_pin_code),
        emergency_contact_name: show_text(&employee.emergency_contact_name),
        emergency_contact_relation: show_text(&employee.emergency_contact_relation),
        emergency_contact_mobile: show_text(&employee.emergency_contact_mobile),
        aadhar_href,
        aadhar_label,
        pan_href,
        pan_label,
        passport_href,
        passport_label,
        account_holder_name: show_text(&employee.account_holder_name),
        account_number: show_text(&employee.account_number),
        account_ifsc_code: show_text(&employee.account_ifsc_code),
        account_type: choice_label(ACCOUNT_TYPE_CHOICES, employee.account_type.as_deref()),
        qualifications: show_text(&employee.qualifications),
        verified: if employee.verified { "Yes" } else { "No" }.to_string(),
        date_of_joining: employee
            .date_of_joining
            .map(format_date)
            .unwrap_or_default(),
        probation_end_date: employee
            .probation_end_date
            .map(format_date)
            .unwrap_or_default(),
        work_start: employee.work_start.map(format_time).unwrap_or_default(),
        work_end: employee.work_end.map(format_time).unwrap_or_default(),
        base_salary: show_money(employee.base_salary),
        hourly_wage: show_money(employee.hourly_wage),
        manager: crate::logic::user::user_label(db, employee.manager_id).await,
    }
}

fn show_text(value: &Option<String>) -> String {
    value.clone().unwrap_or_default()
}

fn show_money(value: Option<Decimal>) -> String {
    value
        .map(|amount| amount.normalize().to_string())
        .unwrap_or_default()
}

fn choice_label(choices: &[(&str, &str)], value: Option<&str>) -> String {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return String::new();
    };
    choices
        .iter()
        .find(|(key, _)| *key == value)
        .map(|(_, label)| (*label).to_string())
        .unwrap_or_else(|| value.to_string())
}

pub async fn vnode_view(db: &sea_orm::DatabaseConnection, id: Option<i64>) -> (String, String) {
    let Some(id) = id.filter(|id| *id > 0) else {
        return (String::new(), String::new());
    };
    let name =
        lariv_core::web::opt_or_log(VNodeEntity::find_by_id(id).one(db).await, "employee file")
            .map(|node| node.name)
            .unwrap_or_else(|| format!("File #{id}"));
    (
        lariv_plugin_filesystem::routes::VNodeDetailRouteTag::new(id).url(),
        name,
    )
}
