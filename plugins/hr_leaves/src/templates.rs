use lariv_core::components::{SlotCapability, SlotRegistrar};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{TemplateCapability, TemplateOf, TemplateRegistrar};

pub mod leaves;

lariv_core::define_register_items! {
    plugin: HrLeavesTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        LeaveListIdx: LeaveListPageTag => leaves::LeaveListPage,
        LeaveDetailIdx: LeaveDetailPageTag => leaves::LeaveDetailPage,
        LeaveCreateModalIdx: LeaveCreateModalPageTag => leaves::LeaveCreateModalPage,
        LeaveEditModalIdx: LeaveEditModalPageTag => leaves::LeaveEditModalPage,
        LeaveDeleteModalIdx: LeaveDeleteModalPageTag => leaves::LeaveDeleteModalPage,
        LeaveApproveModalIdx: LeaveApproveModalPageTag => leaves::LeaveApproveModalPage,
        LeaveRejectModalIdx: LeaveRejectModalPageTag => leaves::LeaveRejectModalPage,
        LeaveRevokeApprovalModalIdx: LeaveRevokeApprovalModalPageTag => leaves::LeaveRevokeApprovalModalPage,
        LeaveRevokeRejectionModalIdx: LeaveRevokeRejectionModalPageTag => leaves::LeaveRevokeRejectionModalPage,
        EmployeeLeaveJournalIdx: EmployeeLeaveJournalPageTag => leaves::EmployeeLeaveJournalPage,
        GiveLeaveModalIdx: GiveLeaveModalPageTag => leaves::GiveLeaveModalPage,
    ]
}

lariv_core::define_register_items! {
    plugin: HrLeavesTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}
