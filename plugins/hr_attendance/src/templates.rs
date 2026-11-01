use lariv_core::components::{SlotCapability, SlotRegistrar};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{TemplateCapability, TemplateOf, TemplateRegistrar};

pub mod attendances;

lariv_core::define_register_items! {
    plugin: HrAttendanceTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        AttendanceListIdx: AttendanceListPageTag => attendances::AttendanceListPage,
        AttendanceDetailIdx: AttendanceDetailPageTag => attendances::AttendanceDetailPage,
        AttendanceCreateModalIdx: AttendanceCreateModalPageTag => attendances::AttendanceCreateModalPage,
        AttendanceEditModalIdx: AttendanceEditModalPageTag => attendances::AttendanceEditModalPage,
        AttendanceDeleteModalIdx: AttendanceDeleteModalPageTag => attendances::AttendanceDeleteModalPage,
    ]
}

lariv_core::define_register_items! {
    plugin: HrAttendanceTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}
