//! Compile a `.typ` VNode to PDF via Typst and preview it in a modal.

use axum::{
    body::Body,
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use maud::{Markup, html};

use crate::{
    components::{ButtonDownload, button_download, modal_keyed},
    http::Cap,
    plugins::{
        filesystem::{
            keys::VNodePdfModalKey, node, routes::VNodePdfRouteTag, state::FilesystemState,
            zip::read_file_bytes,
        },
        users::middleware::RequireAuth,
    },
};

fn pdf_filename(typ_name: &str) -> String {
    let stem = typ_name
        .get(..typ_name.len().saturating_sub(4))
        .filter(|_| typ_name.to_ascii_lowercase().ends_with(".typ"))
        .filter(|s| !s.is_empty())
        .unwrap_or(typ_name);
    format!("{stem}.pdf")
}

fn render_pdf_modal(title: &str, pdf_url: &str) -> Markup {
    modal_keyed::<VNodePdfModalKey>(
        "max-w-6xl w-[95vw]",
        html! {
            div class="flex items-center justify-between gap-3 mb-3 pr-10" {
                h3 class="text-lg font-semibold" { (title) }
                (button_download(ButtonDownload {
                    label: "Download",
                    href: pdf_url,
                    classes: "btn-outline btn-sm",
                    ..Default::default()
                }))
            }
            div class="relative w-full h-[75vh]" x-data="{ loading: true }" {
                div
                    class="absolute inset-0 z-10 flex flex-col items-center justify-center gap-3 rounded border border-base-300 bg-base-100"
                    x-show="loading"
                    x-cloak
                {
                    span class="loading loading-spinner loading-lg" {}
                    p class="text-sm opacity-70" { "Generating PDF…" }
                }
                iframe
                    src=(pdf_url)
                    class="w-full h-full border border-base-300 rounded bg-white"
                    title=(title)
                    x-on:load="loading = false" {}
            }
        },
    )
}

fn render_pdf_modal_error(message: &str) -> Markup {
    modal_keyed::<VNodePdfModalKey>(
        "max-w-2xl",
        html! {
            h3 class="text-lg font-semibold mb-2" { "Typst PDF failed" }
            p class="text-error whitespace-pre-wrap" { (message) }
        },
    )
}

fn pdf_ok_response(filename: &str, bytes: Vec<u8>) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("inline; filename=\"{filename}\""),
            ),
        ],
        Body::from(bytes),
    )
        .into_response()
}

/// GET modal: show the compiled Typst PDF in an iframe.
pub async fn pdf_modal(
    Cap(state): Cap<FilesystemState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Markup {
    let Some(n) = crate::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
    else {
        return render_pdf_modal_error("File not found");
    };
    if !node::is_typst_file(&n.name, n.is_directory) {
        return render_pdf_modal_error("Only .typ files can be rendered as PDF");
    }
    render_pdf_modal(
        &format!("{} PDF", n.name),
        &VNodePdfRouteTag::new(id).path(),
    )
}

/// GET: compile the VNode's Typst source and return PDF bytes.
pub async fn pdf_file(
    Cap(state): Cap<FilesystemState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    let Some(n) = crate::web::opt_or_log(node::get_by_id(&state.db, id).await, "get node by id")
    else {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    };
    if !node::is_typst_file(&n.name, n.is_directory) {
        return (
            StatusCode::BAD_REQUEST,
            "Only .typ files can be rendered as PDF",
        )
            .into_response();
    }
    let bytes = match read_file_bytes(state.store.as_ref(), &n).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, vnode_id = id, "read typst vnode");
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };
    let source = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "File is not valid UTF-8").into_response();
        }
    };
    match crate::typst::typst_compile(&source).await {
        Ok(pdf) => pdf_ok_response(&pdf_filename(&n.name), pdf),
        Err(msg) => {
            tracing::error!(error = %msg, vnode_id = id, "typst compile");
            (StatusCode::BAD_REQUEST, msg).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::pdf_filename;

    #[test]
    fn pdf_filename_replaces_typ_suffix() {
        assert_eq!(pdf_filename("notes.typ"), "notes.pdf");
        assert_eq!(pdf_filename("Notes.TYP"), "Notes.pdf");
        assert_eq!(pdf_filename("a.typ"), "a.pdf");
    }
}
