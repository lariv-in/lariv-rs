//! SeaORM entities for documents and type-specific tables.

pub mod aadhar_card;
pub mod document;
pub mod document_preferences;
pub mod pan_card;
pub mod passport;

pub use aadhar_card::Entity as AadharCardEntity;
pub use aadhar_card::Model as AadharCard;
pub use document::Entity as DocumentEntity;
pub use document::Model as Document;
pub use pan_card::Entity as PanCardEntity;
pub use pan_card::Model as PanCard;
pub use passport::Entity as PassportEntity;
pub use passport::Model as Passport;
