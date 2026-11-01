use lariv_core::components::{SlotCapability, SlotRegistrar};
use lariv_core::http::ProvideRequestCaps;
use lariv_core::template::{TemplateCapability, TemplateOf, TemplateRegistrar};

pub mod holidays;

lariv_core::define_register_items! {
    plugin: HrHolidaysTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    wrapper: TemplateOf;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    hook: Hook;
    items: [
        HolidayListIdx: HolidayListPageTag => holidays::HolidayListPage,
        HolidayDetailIdx: HolidayDetailPageTag => holidays::HolidayDetailPage,
        HolidayCreateModalIdx: HolidayCreateModalPageTag => holidays::HolidayCreateModalPage,
        HolidayEditModalIdx: HolidayEditModalPageTag => holidays::HolidayEditModalPage,
        HolidayDeleteModalIdx: HolidayDeleteModalPageTag => holidays::HolidayDeleteModalPage,
    ]
}

lariv_core::define_register_items! {
    plugin: HrHolidaysTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    bounds: [];
    items: [];
    hook: SlotsHook;
}
