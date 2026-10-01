use chrono::NaiveDate;
use sea_orm::ActiveValue::Set;
use sea_orm::EntityTrait;

use crate::datetime::{format_date, parse_date};
use crate::html_form::{HtmlForm, UploadedFile};
use crate::plugins::filesystem::entities::filesystem_node::Entity as VNodeEntity;
use crate::plugins::filesystem::node::{self, NodeFile};
use crate::plugins::filesystem::state::FilesystemState;
use crate::plugins::hr::blood_group::BloodGroup;
use crate::plugins::hr::entities::employee::{self, ActiveModel as EmployeeActive};
use crate::plugins::hr::forms::{ACCOUNT_TYPE_CHOICES, EmployeeForm, MARITAL_STATUS_CHOICES};
use crate::plugins::hr::gender::ApplicantGender;

pub const HR_EMPLOYEES_DIR: &str = "HR Employees";

#[derive(Clone, Debug)]
pub struct EmployeeProfile {
    pub fathers_name: String,
    pub date_of_birth: Option<NaiveDate>,
    pub gender: Option<ApplicantGender>,
    pub marital_status: String,
    pub nationality: String,
    pub is_disabled: bool,
    pub disability_type: Option<String>,
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
    pub aadhar_vnode_id: Option<i64>,
    pub pan_vnode_id: Option<i64>,
    pub passport_vnode_id: Option<i64>,
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
            disability_type: None,
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
            aadhar_vnode_id: None,
            pan_vnode_id: None,
            passport_vnode_id: None,
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
    am.aadhar_vnode_id = Set(profile.aadhar_vnode_id);
    am.pan_vnode_id = Set(profile.pan_vnode_id);
    am.passport_vnode_id = Set(profile.passport_vnode_id);
    am.account_holder_name = Set(profile.account_holder_name.clone());
    am.account_number = Set(profile.account_number.clone());
    am.account_ifsc_code = Set(profile.account_ifsc_code.clone());
    am.account_type = Set(profile.account_type.clone());
    am.qualifications = Set(profile.qualifications.clone());
    am.date_of_joining = Set(profile.date_of_joining);
    am.probation_end_date = Set(profile.probation_end_date);
}

pub async fn store_employee_file(
    fs: &FilesystemState,
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

    let vnode = node::create(
        &fs.db,
        fs.store.as_ref(),
        name,
        false,
        Some(NodeFile::Upload(file)),
        Some(&parent),
        None,
        None,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(vnode.id)
}

pub async fn profile_from_submit(
    fs: &FilesystemState,
    employee_name: &str,
    mut submit: <EmployeeForm as HtmlForm>::Submit,
    existing: Option<&employee::Model>,
) -> Result<EmployeeProfile, String> {
    let gender = if submit.gender.trim().is_empty() {
        None
    } else {
        Some(ApplicantGender::parse(&submit.gender).ok_or_else(|| "Choose a gender".to_string())?)
    };
    let marital_status =
        choice_or_empty(&submit.marital_status, MARITAL_STATUS_CHOICES, "marital status")?;
    let blood_group = if submit.blood_group.trim().is_empty() {
        None
    } else {
        Some(
            BloodGroup::parse(&submit.blood_group)
                .ok_or_else(|| "Choose a blood group".to_string())?,
        )
    };
    let nationality = choice_or_empty(
        &submit.nationality,
        crate::plugins::hr::countries::ALL_COUNTRIES,
        "country",
    )?;
    let disability_type = if submit.is_disabled {
        let dt = choice_or_empty(
            &submit.disability_type,
            crate::plugins::hr::disability::REGISTERED_DISABILITIES_INDIA,
            "disability type",
        )?;
        if dt.is_empty() {
            None
        } else {
            Some(dt)
        }
    } else {
        None
    };
    let account_type = choice_or_empty(&submit.account_type, ACCOUNT_TYPE_CHOICES, "account type")?;

    // Create must receive identity documents — Option<Upload> otherwise becomes None with
    // no error when multipart/HTMX drops file parts.
    if existing.is_none() {
        let mut missing = Vec::new();
        if submit.photograph.is_none() {
            missing.push("photograph");
        }
        if submit.aadhar.is_none() {
            missing.push("Aadhar");
        }
        if submit.pan.is_none() {
            missing.push("PAN");
        }
        if !missing.is_empty() {
            return Err(format!(
                "Upload required: {}. Files were missing from the submission — re-select them and try again.",
                missing.join(", ")
            ));
        }
    }

    let photograph_vnode_id = match submit.photograph.take() {
        Some(file) => Some(store_employee_file(fs, employee_name, "photograph", file).await?),
        None => existing.and_then(|e| e.photograph_vnode_id),
    };
    let aadhar_vnode_id = match submit.aadhar.take() {
        Some(file) => Some(store_employee_file(fs, employee_name, "aadhar", file).await?),
        None => existing.and_then(|e| e.aadhar_vnode_id),
    };
    let pan_vnode_id = match submit.pan.take() {
        Some(file) => Some(store_employee_file(fs, employee_name, "pan", file).await?),
        None => existing.and_then(|e| e.pan_vnode_id),
    };
    let passport_vnode_id = match submit.passport.take() {
        Some(file) => Some(store_employee_file(fs, employee_name, "passport", file).await?),
        None => existing.and_then(|e| e.passport_vnode_id),
    };

    let (permanent_address, permanent_pin_code) = if submit.same_as_present {
        (
            submit.present_address.trim().to_string(),
            submit.present_pin_code.trim().to_string(),
        )
    } else {
        (
            submit.permanent_address.trim().to_string(),
            submit.permanent_pin_code.trim().to_string(),
        )
    };

    Ok(EmployeeProfile {
        fathers_name: submit.fathers_name.trim().to_string(),
        date_of_birth: optional_date(&submit.date_of_birth, "Date of birth")?,
        gender,
        marital_status,
        nationality,
        is_disabled: submit.is_disabled,
        disability_type,
        photograph_vnode_id,
        blood_group,
        identification_mark: submit.identification_mark.trim().to_string(),
        present_address: submit.present_address.trim().to_string(),
        present_pin_code: submit.present_pin_code.trim().to_string(),
        permanent_address,
        permanent_pin_code,
        emergency_contact_name: submit.emergency_contact_name.trim().to_string(),
        emergency_contact_relation: submit.emergency_contact_relation.trim().to_string(),
        emergency_contact_mobile: submit.emergency_contact_mobile.trim().to_string(),
        aadhar_vnode_id,
        pan_vnode_id,
        passport_vnode_id,
        account_holder_name: submit.account_holder_name.trim().to_string(),
        account_number: submit.account_number.trim().to_string(),
        account_ifsc_code: submit.account_ifsc_code.trim().to_string(),
        account_type,
        qualifications: submit.qualifications.trim().to_string(),
        date_of_joining: optional_date(&submit.date_of_joining, "Date of joining")?,
        probation_end_date: optional_date(&submit.probation_end_date, "Probation end date")?,
    })
}

pub async fn profile_from_form(
    _db: &sea_orm::DatabaseConnection,
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
    let nationality = choice_or_empty(
        &form.nationality,
        crate::plugins::hr::countries::ALL_COUNTRIES,
        "country",
    )?;
    let disability_type = if form.is_disabled {
        let dt = choice_or_empty(
            &form.disability_type,
            crate::plugins::hr::disability::REGISTERED_DISABILITIES_INDIA,
            "disability type",
        )?;
        if dt.is_empty() {
            None
        } else {
            Some(dt)
        }
    } else {
        None
    };
    let account_type = choice_or_empty(&form.account_type, ACCOUNT_TYPE_CHOICES, "account type")?;
    let (permanent_address, permanent_pin_code) = if form.same_as_present {
        (
            form.present_address.trim().to_string(),
            form.present_pin_code.trim().to_string(),
        )
    } else {
        (
            form.permanent_address.trim().to_string(),
            form.permanent_pin_code.trim().to_string(),
        )
    };
    Ok(EmployeeProfile {
        fathers_name: form.fathers_name.trim().to_string(),
        date_of_birth: optional_date(&form.date_of_birth, "Date of birth")?,
        gender,
        marital_status,
        nationality,
        is_disabled: form.is_disabled,
        disability_type,
        photograph_vnode_id: None,
        blood_group,
        identification_mark: form.identification_mark.trim().to_string(),
        present_address: form.present_address.trim().to_string(),
        present_pin_code: form.present_pin_code.trim().to_string(),
        permanent_address,
        permanent_pin_code,
        emergency_contact_name: form.emergency_contact_name.trim().to_string(),
        emergency_contact_relation: form.emergency_contact_relation.trim().to_string(),
        emergency_contact_mobile: form.emergency_contact_mobile.trim().to_string(),
        aadhar_vnode_id: None,
        pan_vnode_id: None,
        passport_vnode_id: None,
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
    let (aadhar_href, aadhar_label) = vnode_view(db, employee.aadhar_vnode_id).await;
    let (pan_href, pan_label) = vnode_view(db, employee.pan_vnode_id).await;
    let (passport_href, passport_label) = vnode_view(db, employee.passport_vnode_id).await;
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
        disability_type: employee.disability_type.clone().unwrap_or_default(),
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

pub async fn vnode_view(db: &sea_orm::DatabaseConnection, id: Option<i64>) -> (String, String) {
    let Some(id) = id.filter(|id| *id > 0) else {
        return (String::new(), String::new());
    };
    let name = crate::web::opt_or_log(VNodeEntity::find_by_id(id).one(db).await, "employee file")
        .map(|node| node.name)
        .unwrap_or_else(|| format!("File #{id}"));
    (
        crate::plugins::filesystem::routes::VNodeDetailRouteTag::new(id).url(),
        name,
    )
}
