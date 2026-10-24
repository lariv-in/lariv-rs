pub mod service;
pub mod service_journal_entry_item_template;
pub mod service_preferences;
pub mod service_preferences_tax;
pub mod service_tax;

pub use service::Entity as ServiceEntity;
pub use service_journal_entry_item_template::Entity as ServiceJournalEntryItemTemplateEntity;
pub use service_preferences::Entity as ServicePreferencesEntity;
pub use service_tax::Entity as ServiceTaxEntity;
