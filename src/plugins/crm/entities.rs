pub mod company;
pub mod contact;
pub mod converted_lead;
pub mod deal;
pub mod failed_lead;
pub mod lead;

pub use company::Entity as CompanyEntity;
pub use contact::Entity as ContactEntity;
pub use converted_lead::Entity as ConvertedLeadEntity;
pub use deal::Entity as DealEntity;
pub use failed_lead::Entity as FailedLeadEntity;
pub use lead::Entity as LeadEntity;
