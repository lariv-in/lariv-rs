//! SeaORM entities for documents and type-specific tables.

pub mod aadhar_card;
pub mod document;
pub mod document_preferences;

pub use aadhar_card::Entity as AadharCardEntity;
pub use aadhar_card::Model as AadharCard;
pub use document::Entity as DocumentEntity;
pub use document::Model as Document;
