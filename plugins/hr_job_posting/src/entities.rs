//! Job posting entities live in the HR hub (historical migrations); re-export for local use.
pub use lariv_plugin_hr::entities::applicant;
pub use lariv_plugin_hr::entities::job_form;

pub use applicant::Entity as ApplicantEntity;
pub use job_form::Entity as JobFormEntity;
