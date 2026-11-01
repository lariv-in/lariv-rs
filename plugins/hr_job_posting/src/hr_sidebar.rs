//! Applicants and job postings patched onto the shared HR app sidebar.

use lariv_plugin_hr::hr_sidebar::{HrSidebarRegistrar, HrSidebarRegistry, link_with_prefixes};
use lariv_plugin_hr::routes::HrPeopleView;
use lariv_plugin_users::role_authorization::roles_for;

use crate::routes::{ApplicantListRouteTag, JobFormListRouteTag, JobFormView};

const APPLICANTS_MATCH_PREFIXES: &[&str] = &[ApplicantListRouteTag::PATH];
const JOB_FORMS_MATCH_PREFIXES: &[&str] = &[JobFormListRouteTag::PATH];

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl HrSidebarRegistrar for Hook {
    fn register_hr_sidebar(self, cap: HrSidebarRegistry) -> HrSidebarRegistry {
        cap.push(
            link_with_prefixes::<ApplicantListRouteTag>(
                "applicants",
                "Applicants",
                15,
                None,
                APPLICANTS_MATCH_PREFIXES,
            )
            .with_roles(roles_for::<HrPeopleView>),
        )
        .push(
            link_with_prefixes::<JobFormListRouteTag>(
                "job-forms",
                "Job postings",
                20,
                None,
                JOB_FORMS_MATCH_PREFIXES,
            )
            .with_roles(roles_for::<JobFormView>),
        )
    }
}
