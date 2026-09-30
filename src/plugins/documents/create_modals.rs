//! Typed [`CreateModal`] wiring for the document create button.

use super::keys::DocumentCreateModalKey;
use super::routes::{DocumentCreateGetRouteTag, DocumentCreatePostRouteTag};

crate::impl_create_modal!(
    DocumentCreateModalKey,
    DocumentCreateGetRouteTag,
    DocumentCreatePostRouteTag,
    "p_documents.DocumentCreateForm"
);
