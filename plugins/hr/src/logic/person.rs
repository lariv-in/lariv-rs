#[derive(Clone)]
pub struct PersonInput {
    pub name: String,
    pub mobile: String,
    pub email: String,
}

pub fn validate_person_input(input: &PersonInput) -> Result<(), String> {
    if input.name.trim().is_empty() {
        return Err("name is required".to_string());
    }
    if input.mobile.trim().is_empty() {
        return Err("mobile is required".to_string());
    }
    if input.email.trim().is_empty() {
        return Err("email is required".to_string());
    }
    Ok(())
}

pub fn normalized_person_input(input: &PersonInput) -> PersonInput {
    PersonInput {
        name: input.name.trim().to_string(),
        mobile: input.mobile.trim().to_string(),
        email: input.email.trim().to_string(),
    }
}

pub fn person_display_name(name: &str, id: i64, fallback_label: &str) -> String {
    if name.trim().is_empty() {
        format!("{fallback_label} #{id}")
    } else {
        name.to_string()
    }
}
