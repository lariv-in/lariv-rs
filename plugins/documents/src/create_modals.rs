//! Typed [`CreateModal`] wiring for the document create button.

use super::keys::DocumentCreateModalKey;
use super::routes::{DocumentCreateGetRouteTag, DocumentCreatePostRouteTag};

lariv_core::impl_create_modal!(
    DocumentCreateModalKey,
    DocumentCreateGetRouteTag,
    DocumentCreatePostRouteTag,
    "p_documents.DocumentCreateForm"
);
