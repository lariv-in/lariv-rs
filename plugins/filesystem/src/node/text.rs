//! Filename / editor / text-content helpers.

use tokio::io::AsyncReadExt;

use crate::entities::VNode;
use crate::storage::DynFilestore;

/// Max bytes loaded into the in-browser text editor (or accepted on save).
pub const MAX_TEXT_BYTES: usize = 1024 * 1024;

/// trims whitespace and strips any directory
/// components, rejecting `.`/`..`.
pub fn sanitize_node_name(name: &str) -> String {
    let trimmed = name.trim();
    let candidate = trimmed
        .rsplit(['/', '\\'])
        .find(|s| !s.is_empty())
        .unwrap_or("");
    if candidate == "." || candidate == ".." {
        String::new()
    } else {
        candidate.to_string()
    }
}

/// `"a.txt"` → `".txt"`, `"a"` → `""`.
pub fn ext_of(filename: &str) -> String {
    std::path::Path::new(filename)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default()
}

/// True for a file whose name ends with `.typ` (Typst source).
pub fn is_typst_file(name: &str, is_directory: bool) -> bool {
    !is_directory && name.to_ascii_lowercase().ends_with(".typ")
}

/// CodeMirror language key for a VNode filename.
pub fn editor_language(name: &str) -> &'static str {
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "md" | "markdown" => "markdown",
        "js" | "mjs" | "cjs" => "javascript",
        "typ" => "typst",
        _ => "plaintext",
    }
}

/// UTF-8 text within [`MAX_TEXT_BYTES`].
pub fn decode_text(bytes: &[u8]) -> Option<&str> {
    if bytes.len() > MAX_TEXT_BYTES {
        return None;
    }
    std::str::from_utf8(bytes).ok()
}

/// Error unless `content` is within [`MAX_TEXT_BYTES`].
pub fn require_text_content_size(content: &str) -> Result<(), String> {
    if content.len() > MAX_TEXT_BYTES {
        return Err(format!(
            "content is {} bytes; text writes are limited to {MAX_TEXT_BYTES} bytes",
            content.len()
        ));
    }
    Ok(())
}

/// Load editor text for a file VNode, or `None` for directories / binary / oversized files.
///
/// Missing blobs and files with no stored path are treated as empty text so the editor
/// can create content.
pub async fn try_read_text(store: &DynFilestore, node: &VNode) -> Option<String> {
    if node.is_directory {
        return None;
    }
    let Some(path) = node.file_path.as_deref().filter(|p| !p.is_empty()) else {
        return Some(String::new());
    };
    match store.stored_size(path).await {
        Ok(size) if size as usize > MAX_TEXT_BYTES => return None,
        Err(e) if e.is_missing() => return Some(String::new()),
        Err(_) => return None,
        Ok(_) => {}
    }
    let mut download = store.open(path, &node.name).await.ok()?;
    let mut buf = Vec::new();
    download.reader.read_to_end(&mut buf).await.ok()?;
    decode_text(&buf).map(str::to_string)
}

pub fn item_type(node: &VNode) -> &'static str {
    if node.is_directory {
        "Directory"
    } else {
        "File"
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_TEXT_BYTES, decode_text, editor_language, is_typst_file, require_text_content_size,
    };

    #[test]
    fn typst_file_detects_typ_extension() {
        assert!(is_typst_file("notes.typ", false));
        assert!(is_typst_file("Notes.TYP", false));
        assert!(!is_typst_file("notes.typ", true));
        assert!(!is_typst_file("notes.txt", false));
        assert!(!is_typst_file("notes.typst", false));
    }

    #[test]
    fn editor_language_maps_known_extensions() {
        assert_eq!(editor_language("readme.md"), "markdown");
        assert_eq!(editor_language("README.MARKDOWN"), "markdown");
        assert_eq!(editor_language("app.js"), "javascript");
        assert_eq!(editor_language("mod.mjs"), "javascript");
        assert_eq!(editor_language("lib.cjs"), "javascript");
        assert_eq!(editor_language("notes.typ"), "typst");
        assert_eq!(editor_language("Notes.TYP"), "typst");
        assert_eq!(editor_language("notes.txt"), "plaintext");
        assert_eq!(editor_language("noext"), "plaintext");
    }

    #[test]
    fn decode_text_accepts_utf8() {
        assert_eq!(decode_text(b"hello").unwrap(), "hello");
        assert_eq!(decode_text("café".as_bytes()).unwrap(), "café");
        assert_eq!(decode_text(b"").unwrap(), "");
    }

    #[test]
    fn decode_text_rejects_binary() {
        assert!(decode_text(&[0xff, 0xfe, 0x00]).is_none());
    }

    #[test]
    fn decode_text_rejects_oversize() {
        let bytes = vec![b'a'; MAX_TEXT_BYTES + 1];
        assert!(decode_text(&bytes).is_none());
    }

    #[test]
    fn require_text_content_size_rejects_oversize() {
        let content = "a".repeat(MAX_TEXT_BYTES + 1);
        let err = require_text_content_size(&content).unwrap_err();
        assert!(err.contains("limited to"));
        assert!(require_text_content_size("ok").is_ok());
    }
}
