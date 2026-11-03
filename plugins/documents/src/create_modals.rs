//! Typed [`CreateModal`] wiring for the document create button.

use lariv_core::web::CreateModal;

use super::keys::DOCUMENT_CREATE_MODAL;
use super::routes::{DocumentCreateGetRouteTag, DocumentCreatePostRouteTag};

pub const DOCUMENT_CREATE: CreateModal<DocumentCreateGetRouteTag, DocumentCreatePostRouteTag> =
    CreateModal {
        key: DOCUMENT_CREATE_MODAL,
        form_name: "p_documents.DocumentCreateForm",
        get: DocumentCreateGetRouteTag,
        post: DocumentCreatePostRouteTag,
    };
