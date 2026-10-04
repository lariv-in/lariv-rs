//! HR plugin compile smoke test.

#![recursion_limit = "512"]

use std::path::PathBuf;

use lariv_rs::app::App;
use lariv_rs::plugins::{dashboard, documents, filesystem, forms, hr, otp, users, website};

const MINIMAL_DB_TOML: &str = r#"database_url = "sqlite::memory:""#;

fn temp_config(name: &str, body: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lariv-hr-compile-{name}-{}-{}.toml",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::write(&path, body).expect("write temp config");
    path
}

#[test]
fn hr_plugin_mounts() {
    std::thread::Builder::new()
        .name("hr-plugin-mount".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async {
                let app = App::new_web_app();
                let app = users::install(app);
                let app = otp::install(app);
                let app = filesystem::install(app);
                let app = forms::install(app);
                let app = website::install(app);
                let app = documents::install(app);
                let app = dashboard::install(app);
                let app = hr::install(app);
                let path = temp_config("db", MINIMAL_DB_TOML);
                let app = app.load_config(&path).await.expect("load_config");
                std::fs::remove_file(&path).ok();
                let _mounted = app.mount();
            });
        })
        .expect("spawn hr-plugin-mount thread")
        .join()
        .expect("hr-plugin-mount thread");
}

#[test]
fn test_employee_form_specs() {
    use lariv_rs::html_form::{FormFieldKey, HtmlForm};
    use lariv_rs::plugins::hr::forms::{EmployeeForm, EmployeeFormField};

    let specs = EmployeeForm::field_specs();
    let find_spec = |field: EmployeeFormField| {
        let name = field.html_name();
        specs
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("missing field spec {name}"))
    };

    // 1. Nationality should have choices and search placeholder
    let nat = find_spec(EmployeeFormField::Nationality);
    assert_eq!(nat.choices_key, Some("nationality"));
    assert_eq!(nat.placeholder, Some("Search country…"));
    assert!(
        EmployeeForm::nationality_choices()
            .iter()
            .any(|(k, _)| *k == "India")
    );
    assert!(EmployeeForm::nationality_choices().len() > 190);

    // 2. Disability type should be conditionally shown on is_disabled and searchable combobox
    let is_disabled = find_spec(EmployeeFormField::IsDisabled);
    assert_eq!(is_disabled.model, Some("is_disabled"));

    let dis = find_spec(EmployeeFormField::DisabilityType);
    assert_eq!(dis.choices_key, Some("disability_type"));
    assert_eq!(dis.show, Some("is_disabled"));
    assert_eq!(dis.placeholder, Some("Search disability…"));
    let dis_choices = EmployeeForm::disability_type_choices();
    assert_eq!(dis_choices.len(), 21);
    assert!(dis_choices.iter().any(|(k, _)| *k == "Acid Attack victim"));
    assert!(
        dis_choices
            .iter()
            .any(|(k, _)| *k == "Locomotor Disability")
    );
    assert!(
        dis_choices
            .iter()
            .any(|(k, _)| *k == "Autism Spectrum Disorder")
    );

    // 3. Photograph should be a file input, not a FK picker
    let photo = find_spec(EmployeeFormField::Photograph);
    assert_eq!(photo.accept, Some("image/*"));
    assert_eq!(photo.swap_key, None);

    // 4. Same as present checkbox & disabled permanent address/pin code
    let pres_addr = find_spec(EmployeeFormField::PresentAddress);
    assert_eq!(pres_addr.model, Some("present_address"));

    let pres_pin = find_spec(EmployeeFormField::PresentPinCode);
    assert_eq!(pres_pin.model, Some("present_pin_code"));

    let same_as = find_spec(EmployeeFormField::SameAsPresent);
    assert_eq!(same_as.model, Some("same_as_present"));

    let perm_addr = find_spec(EmployeeFormField::PermanentAddress);
    assert_eq!(perm_addr.model, Some("permanent_address"));
    assert_eq!(perm_addr.disabled, Some("same_as_present"));

    let perm_pin = find_spec(EmployeeFormField::PermanentPinCode);
    assert_eq!(perm_pin.model, Some("permanent_pin_code"));
    assert_eq!(perm_pin.disabled, Some("same_as_present"));

    // 5. Aadhar, PAN, Passport should be file inputs
    let aadhar = find_spec(EmployeeFormField::Aadhar);
    assert_eq!(aadhar.accept, Some(".pdf,.jpg,.jpeg,.png"));
    assert_eq!(aadhar.swap_key, None);

    let pan = find_spec(EmployeeFormField::Pan);
    assert_eq!(pan.accept, Some(".pdf,.jpg,.jpeg,.png"));
    assert_eq!(pan.swap_key, None);

    let passport = find_spec(EmployeeFormField::Passport);
    assert_eq!(passport.accept, Some(".pdf,.jpg,.jpeg,.png"));
    assert_eq!(passport.swap_key, None);

    // 6. Prepend Bank to Account fields
    let holder = find_spec(EmployeeFormField::AccountHolderName);
    assert_eq!(holder.label, "Bank account holder name");

    let num = find_spec(EmployeeFormField::AccountNumber);
    assert_eq!(num.label, "Bank account number");

    let ifsc = find_spec(EmployeeFormField::AccountIfscCode);
    assert_eq!(ifsc.label, "Bank account IFSC code");

    let ty = find_spec(EmployeeFormField::AccountType);
    assert_eq!(ty.label, "Bank account type");
}

#[tokio::test]
async fn test_employee_profile_logic() {
    use lariv_rs::plugins::hr::forms::EmployeeForm;
    use lariv_rs::plugins::hr::logic::profile::profile_from_form;
    use sea_orm::Database;

    let db = Database::connect("sqlite::memory:")
        .await
        .expect("db connect");

    // Test with same_as_present = true
    let form = EmployeeForm {
        name: "Test User".into(),
        mobile: "9876543210".into(),
        email: "test@example.com".into(),
        fathers_name: "Father".into(),
        date_of_birth: "".into(),
        gender: "".into(),
        marital_status: "".into(),
        nationality: "India".into(),
        is_disabled: false,
        disability_type: "Locomotor Disability".into(),
        photograph: None,
        blood_group: "".into(),
        identification_mark: "".into(),
        present_address: "123 Main St".into(),
        present_pin_code: "560001".into(),
        same_as_present: true,
        permanent_address: "Old Address".into(),
        permanent_pin_code: "110001".into(),
        emergency_contact_name: "".into(),
        emergency_contact_relation: "".into(),
        emergency_contact_mobile: "".into(),
        aadhar: None,
        pan: None,
        passport: None,
        account_holder_name: "Test User".into(),
        account_number: "12345678".into(),
        account_ifsc_code: "SBIN0001234".into(),
        account_type: "savings".into(),
        qualifications: "".into(),
        date_of_joining: "".into(),
        probation_end_date: "".into(),
        work_start: "".into(),
        work_end: "".into(),
        base_salary: "".into(),
        hourly_wage: "".into(),
        csrf: Default::default(),
    };

    let profile = profile_from_form(&db, &form)
        .await
        .expect("profile_from_form");
    // When is_disabled is false, disability_type is None
    assert_eq!(profile.disability_type, None);
    // When same_as_present is true, permanent address/pin are synced from present
    assert_eq!(profile.permanent_address.as_deref(), Some("123 Main St"));
    assert_eq!(profile.permanent_pin_code.as_deref(), Some("560001"));
    assert_eq!(profile.nationality.as_deref(), Some("India"));
    assert_eq!(profile.fathers_name.as_deref(), Some("Father"));

    // Test with is_disabled = true
    let form2 = EmployeeForm {
        name: "Test User".into(),
        mobile: "9876543210".into(),
        email: "test@example.com".into(),
        fathers_name: "Father".into(),
        date_of_birth: "".into(),
        gender: "".into(),
        marital_status: "".into(),
        nationality: "India".into(),
        is_disabled: true,
        disability_type: "Locomotor Disability".into(),
        photograph: None,
        blood_group: "".into(),
        identification_mark: "".into(),
        present_address: "123 Main St".into(),
        present_pin_code: "560001".into(),
        same_as_present: false,
        permanent_address: "456 Other St".into(),
        permanent_pin_code: "110001".into(),
        emergency_contact_name: "".into(),
        emergency_contact_relation: "".into(),
        emergency_contact_mobile: "".into(),
        aadhar: None,
        pan: None,
        passport: None,
        account_holder_name: "Test User".into(),
        account_number: "12345678".into(),
        account_ifsc_code: "SBIN0001234".into(),
        account_type: "savings".into(),
        qualifications: "".into(),
        date_of_joining: "".into(),
        probation_end_date: "".into(),
        work_start: "09:00".into(),
        work_end: "18:00:00".into(),
        base_salary: "25000.50".into(),
        hourly_wage: "120".into(),
        csrf: Default::default(),
    };

    let profile2 = profile_from_form(&db, &form2)
        .await
        .expect("profile_from_form disabled");
    assert_eq!(
        profile2.disability_type,
        Some("Locomotor Disability".into())
    );
    assert_eq!(profile2.permanent_address.as_deref(), Some("456 Other St"));
    assert_eq!(profile2.permanent_pin_code.as_deref(), Some("110001"));
    assert_eq!(
        profile2.work_start.map(|t| t.format("%H:%M").to_string()),
        Some("09:00".into())
    );
    assert_eq!(
        profile2.work_end.map(|t| t.format("%H:%M:%S").to_string()),
        Some("18:00:00".into())
    );
    assert_eq!(
        profile2.base_salary.map(|amount| amount.to_string()),
        Some("25000.50".into())
    );
    assert_eq!(
        profile2.hourly_wage.map(|amount| amount.to_string()),
        Some("120".into())
    );
}

#[tokio::test]
async fn test_employee_templates_rendering() {
    use lariv_rs::plugins::hr::forms::EmployeeFormField;
    use lariv_rs::plugins::hr::templates::{
        EmployeeFormValues, PersonCreateKind, PersonCreateModalPage,
    };
    use lariv_rs::template::RenderTemplate;

    let mut values = EmployeeFormValues::default();
    values.photograph_display = "photo.jpg".into();
    values.aadhar_display = "aadhar.pdf".into();
    values.pan_display = "pan.pdf".into();
    values.passport_display = "passport.pdf".into();

    let page = PersonCreateModalPage::with_employee(
        "hr_employee_create".into(),
        "hr_table".into(),
        "New Employee".into(),
        "Save".into(),
        PersonCreateKind::Employee,
        values,
        "".into(),
    );

    let chrome = lariv_rs::components::ShellChrome::default();
    let html = page.render(&chrome).into_string();

    // Verify multipart enctype & hx-encoding
    assert!(
        html.contains(r#"enctype="multipart/form-data""#),
        "missing enctype: {html}"
    );
    assert!(
        html.contains(r#"hx-encoding="multipart/form-data""#),
        "missing hx-encoding: {html}"
    );

    // Verify file input types (not text / foreign key)
    use lariv_rs::html_form::FormFieldKey;
    assert!(html.contains(&format!(
        r#"name="{}""#,
        EmployeeFormField::Photograph.html_name()
    )));
    assert!(html.contains(&format!(
        r#"name="{}""#,
        EmployeeFormField::Aadhar.html_name()
    )));
    assert!(html.contains(&format!(r#"name="{}""#, EmployeeFormField::Pan.html_name())));
    assert!(html.contains(&format!(
        r#"name="{}""#,
        EmployeeFormField::Passport.html_name()
    )));

    // Verify hints for current files
    assert!(html.contains("Current file: photo.jpg"));
    assert!(html.contains("Current file: aadhar.pdf"));
    assert!(html.contains("Current file: pan.pdf"));
    assert!(html.contains("Current file: passport.pdf"));

    // Verify Bank field labels
    assert!(html.contains("Bank account holder name"));
    assert!(html.contains("Bank account number"));
    assert!(html.contains("Bank account IFSC code"));
    assert!(html.contains("Bank account type"));

    // Verify Alpine sync watches
    assert!(html.contains("$watch('same_as_present'"));
    assert!(html.contains("$watch('present_address'"));
    assert!(html.contains("$watch('present_pin_code'"));

    // Verify permanent fields disabled binding
    assert!(html.contains(r#"x-bind:disabled="same_as_present""#));

    // Verify disability_type show binding
    assert!(html.contains(r#"x-show="is_disabled""#));

    // HR staff still see the factual label near the personal details.
    assert!(html.contains("Is disabled"));
    assert!(!html.contains("Do you have any disability"));
    let disabled_at = html
        .find(&format!(
            r#"name="{}""#,
            EmployeeFormField::IsDisabled.html_name()
        ))
        .expect("admin disability checkbox");
    let photo_at = html
        .find(&format!(
            r#"name="{}""#,
            EmployeeFormField::Photograph.html_name()
        ))
        .expect("photograph input");
    assert!(disabled_at < photo_at);
}

#[test]
fn employee_self_service_form_asks_about_disability_last() {
    use lariv_rs::html_form::FormFieldKey;
    use lariv_rs::plugins::hr::forms::EmployeeFormField;
    use lariv_rs::plugins::hr::logic::dashboard::MissingHrProfile;
    use lariv_rs::plugins::hr::templates::{EmployeeFormValues, HrDashboardGatePage};
    use lariv_rs::template::RenderTemplate;

    let page = HrDashboardGatePage::for_employee(
        MissingHrProfile::Employee,
        EmployeeFormValues::default(),
        String::new(),
    );
    let html = page
        .render(&lariv_rs::components::ShellChrome::default())
        .into_string();

    assert!(html.contains("Do you have any disability"));
    assert!(!html.contains("Is disabled"));

    let wage = html
        .find(&format!(
            r#"name="{}""#,
            EmployeeFormField::HourlyWage.html_name()
        ))
        .expect("hourly wage");
    let disability = html
        .find(&format!(
            r#"name="{}""#,
            EmployeeFormField::IsDisabled.html_name()
        ))
        .expect("disability checkbox");
    let disability_type = html
        .find(&format!(
            r#"name="{}""#,
            EmployeeFormField::DisabilityType.html_name()
        ))
        .expect("disability type");
    assert!(wage < disability);
    assert!(disability < disability_type);
}
