use lariv_core::components::{SlotCapability, SlotRegistrar};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{TemplateCapability, TemplateOf, TemplateRegistrar};

pub mod applicants;
pub mod job_forms;

lariv_core::define_register_items! {
    plugin: HrJobPostingTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        ApplicantListIdx: ApplicantListPageTag => applicants::ApplicantListPage,
        ApplicantDetailIdx: ApplicantDetailPageTag => applicants::ApplicantDetailPage,
        ApplicantCreateModalIdx: ApplicantCreateModalPageTag => applicants::ApplicantCreateModalPage,
        ApplicantEditModalIdx: ApplicantEditModalPageTag => applicants::ApplicantEditModalPage,
        ApplicantDeleteModalIdx: ApplicantDeleteModalPageTag => applicants::ApplicantDeleteModalPage,
        HireApplicantModalIdx: HireApplicantModalPageTag => applicants::HireApplicantModalPage,
        JobFormListIdx: JobFormListPageTag => job_forms::JobFormListPage,
        JobFormSelectIdx: JobFormSelectPageTag => job_forms::JobFormSelectPage,
        JobFormDetailIdx: JobFormDetailPageTag => job_forms::JobFormDetailPage,
        JobFormCreateModalIdx: JobFormCreateModalPageTag => job_forms::JobFormCreateModalPage,
        JobFormEditModalIdx: JobFormEditModalPageTag => job_forms::JobFormEditModalPage,
        JobFormDeleteModalIdx: JobFormDeleteModalPageTag => job_forms::JobFormDeleteModalPage,
    ]
}

lariv_core::define_register_items! {
    plugin: HrJobPostingTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}
