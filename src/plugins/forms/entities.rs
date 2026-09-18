//! SeaORM entities for forms and submitted responses.
pub mod form;
pub mod form_response;

pub use form::Entity as FormEntity;
pub use form::Model as Form;
pub use form_response::Entity as FormResponseEntity;
pub use form_response::Model as FormResponse;
