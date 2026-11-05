use lariv_core::components::{SlotCapability, SlotRegistrar};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{TemplateCapability, TemplateOf, TemplateRegistrar};

pub mod overtime;

lariv_core::define_register_items! {
    plugin: HrOvertimeTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        OvertimeListIdx: OvertimeListPageTag => overtime::OvertimeListPage,
        ApprovedOvertimeListIdx: ApprovedOvertimeListPageTag => overtime::ApprovedOvertimeListPage,
        ApprovedOvertimeCreateModalIdx: ApprovedOvertimeCreateModalPageTag => overtime::ApprovedOvertimeCreateModalPage,
        OvertimeDetailIdx: OvertimeDetailPageTag => overtime::OvertimeDetailPage,
        OvertimeCreateModalIdx: OvertimeCreateModalPageTag => overtime::OvertimeCreateModalPage,
        OvertimeEditModalIdx: OvertimeEditModalPageTag => overtime::OvertimeEditModalPage,
        OvertimeDeleteModalIdx: OvertimeDeleteModalPageTag => overtime::OvertimeDeleteModalPage,
        OvertimeApproveModalIdx: OvertimeApproveModalPageTag => overtime::OvertimeApproveModalPage,
        OvertimeRejectModalIdx: OvertimeRejectModalPageTag => overtime::OvertimeRejectModalPage,
        OvertimeRevokeApprovalModalIdx: OvertimeRevokeApprovalModalPageTag => overtime::OvertimeRevokeApprovalModalPage,
        OvertimeRevokeRejectionModalIdx: OvertimeRevokeRejectionModalPageTag => overtime::OvertimeRevokeRejectionModalPage,
    ]
}

lariv_core::define_register_items! {
    plugin: HrOvertimeTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}
