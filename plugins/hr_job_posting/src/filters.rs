//! Catalogs for applicant and job posting table filters.

use lariv_core::list_filters::{FilterField, text_column};

use crate::entities::{applicant, job_form};

pub fn applicant_fields() -> Vec<FilterField> {
    vec![
        text_column("name", "Name", applicant::Column::Name),
        text_column("email", "Email", applicant::Column::Email),
    ]
}

pub fn job_form_fields() -> Vec<FilterField> {
    vec![text_column(
        "title",
        "Job title",
        job_form::Column::JobTitle,
    )]
}

pub use lariv_plugin_hr::filters::{PreparedFilters, prepare};
