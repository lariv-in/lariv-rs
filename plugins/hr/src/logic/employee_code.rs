//! Render an employee code from the preferences template.

use chrono::{DateTime, NaiveDate, Utc};
use minijinja::{Environment, UndefinedBehavior};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};
use serde_json::json;

use crate::entities::employee::{self, Entity as EmployeeEntity};
use crate::logic::preferences::load_employee_id_format;
use lariv_plugin_users::entities::user::{self, Entity as UserEntity};

/// Year of joining, then the user id left-padded with zeros to 5 digits.
/// `employee` and `user` are the stored entities. `employee.date_of_joining`
/// is used when set; otherwise `employee.created_at`.
pub const DEFAULT_EMPLOYEE_ID_FORMAT: &str = "\
{%- set joined = employee.date_of_joining if employee.date_of_joining else employee.created_at -%}
{{- joined[:4] -}}
{{- \"%05d\"|format(user.id) -}}";

pub const EMPLOYEE_ID_FORMAT_HINT: &str = "\
Minijinja template rendered once when an employee is created. The context has two objects: employee (the employee entity) and user (the matching user entity: id, name, email, phone, role, timezone, created_at, updated_at, last_login_time). Leave blank to use the default: the four-digit year of joining, then the user id left-padded with zeros to 5 digits (202600042 for a 2026 joiner whose user id is 42).

Year of joining is employee.date_of_joining when that date is set, otherwise employee.created_at. The default template is:

{% set joined = employee.date_of_joining if employee.date_of_joining else employee.created_at %}{{ joined[:4] }}{{ \"%05d\"|format(user.id) }}";

pub fn validate_employee_id_format(format: &str) -> Result<(), String> {
    if format.trim().is_empty() {
        return Ok(());
    }
    render_employee_code(format, &sample_employee(), &sample_user()).map(|_| ())
}

pub fn render_employee_code(
    format: &str,
    employee: &employee::Model,
    user: &user::Model,
) -> Result<String, String> {
    let format = format.trim();
    let format = if format.is_empty() {
        DEFAULT_EMPLOYEE_ID_FORMAT
    } else {
        format
    };
    let mut user_value = serde_json::to_value(user)
        .map_err(|err| format!("serialize user for employee code: {err}"))?;
    if let Some(obj) = user_value.as_object_mut() {
        obj.remove("password_hash");
        obj.remove("password_salt");
    }
    let employee_value = serde_json::to_value(employee)
        .map_err(|err| format!("serialize employee for employee code: {err}"))?;
    let context = json!({
        "employee": employee_value,
        "user": user_value,
    });
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    let tmpl = env
        .template_from_str(format)
        .map_err(|err| format!("invalid employee ID format: {err}"))?;
    let rendered = tmpl
        .render(context)
        .map_err(|err| format!("employee ID format failed: {err}"))?;
    let code = rendered.trim().to_string();
    if code.is_empty() {
        return Err("employee ID format rendered an empty code".to_string());
    }
    if code.chars().any(char::is_control) {
        return Err(
            "employee ID format rendered a code containing a line break or control character"
                .to_string(),
        );
    }
    Ok(code)
}

/// Fill `employee.employee_code` from the saved format and the employee's user.
pub async fn assign_employee_code<C: ConnectionTrait>(
    db: &C,
    employee: employee::Model,
) -> Result<employee::Model, String> {
    let user = UserEntity::find_by_id(employee.user_id)
        .one(db)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "user not found".to_string())?;
    let format = load_employee_id_format(db).await?;
    let code = render_employee_code(&format, &employee, &user)?;
    let taken = EmployeeEntity::find()
        .filter(employee::Column::EmployeeCode.eq(&code))
        .filter(employee::Column::Id.ne(employee.id))
        .one(db)
        .await
        .map_err(|err| err.to_string())?;
    if taken.is_some() {
        return Err(format!("Employee code {code} is already in use"));
    }
    let mut active: employee::ActiveModel = employee.into();
    active.employee_code = Set(code);
    active.updated_at = Set(Some(Utc::now()));
    active.update(db).await.map_err(|err| err.to_string())
}

fn sample_user() -> user::Model {
    user::Model {
        id: 42,
        created_at: None,
        updated_at: None,
        name: "Ada Lovelace".to_string(),
        email: "ada@example.com".into(),
        phone: "9990001111".into(),
        role: "unassigned".to_string(),
        password_hash: Some(b"secret-hash".to_vec()),
        password_salt: Some(b"secret-salt".to_vec()),
        timezone: "Asia/Kolkata".into(),
        last_login_time: None,
    }
}

fn sample_employee() -> employee::Model {
    let hired_at = DateTime::parse_from_rfc3339("2026-03-02T09:00:00Z")
        .expect("hired at")
        .with_timezone(&Utc);
    employee::Model {
        id: 7,
        created_at: Some(hired_at),
        updated_at: Some(hired_at),
        user_id: 42,
        employee_code: String::new(),
        manager_id: None,
        name: Some("Ada Lovelace".to_string()),
        mobile: Some("9990001111".to_string()),
        email: Some("ada@example.com".to_string()),
        verified: false,
        fathers_name: None,
        date_of_birth: None,
        gender: None,
        marital_status: None,
        nationality: None,
        is_disabled: None,
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
        date_of_joining: Some(NaiveDate::from_ymd_opt(2026, 3, 2).expect("joining")),
        probation_end_date: None,
        work_start: None,
        work_end: None,
        base_salary: None,
        hourly_wage: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_EMPLOYEE_ID_FORMAT, render_employee_code, sample_employee, sample_user,
        validate_employee_id_format,
    };

    #[test]
    fn default_code_is_joining_year_and_padded_user_id() {
        let code = render_employee_code(
            DEFAULT_EMPLOYEE_ID_FORMAT,
            &sample_employee(),
            &sample_user(),
        )
        .expect("code");
        assert_eq!(code, "202600042");
    }

    #[test]
    fn blank_format_uses_the_default() {
        let code = render_employee_code("  ", &sample_employee(), &sample_user()).expect("code");
        assert_eq!(code, "202600042");
    }

    #[test]
    fn missing_joining_date_uses_created_at_year() {
        let mut employee = sample_employee();
        employee.date_of_joining = None;
        employee.created_at = Some(
            chrono::DateTime::parse_from_rfc3339("2024-11-01T00:00:00Z")
                .expect("created")
                .with_timezone(&chrono::Utc),
        );
        let code = render_employee_code("", &employee, &sample_user()).expect("code");
        assert_eq!(code, "202400042");
    }

    #[test]
    fn template_can_read_employee_and_user_fields() {
        let code = render_employee_code(
            "{{ employee.name }}-{{ user.role }}",
            &sample_employee(),
            &sample_user(),
        )
        .expect("code");
        assert_eq!(code, "Ada Lovelace-unassigned");
    }

    #[test]
    fn user_secrets_are_not_in_the_template() {
        let err = render_employee_code(
            "{{ user.password_hash }}",
            &sample_employee(),
            &sample_user(),
        )
        .expect_err("password hash stays out of the template");
        assert!(err.contains("undefined"), "{err}");
    }

    #[test]
    fn invalid_format_is_rejected() {
        let err = validate_employee_id_format("{{ employee.name").expect_err("syntax");
        assert!(err.contains("invalid employee ID format"), "{err}");
    }
}
