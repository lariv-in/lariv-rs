//! Route reference loading and page template split/compose for the GrapesJS builder.

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::plugins::filesystem::{node, storage::DynFilestore};

use super::{
    entities::route_reference::{self, Entity as RouteRefEntity},
    render::read_vnode_text,
};

/// Header/footer ref blobs and their template include paths.
#[derive(Debug, Clone, Default)]
pub struct RouteRefParts {
    pub header_path: Option<String>,
    pub header_src: Option<String>,
    pub footer_path: Option<String>,
    pub footer_src: Option<String>,
}

/// Middle content extracted from a page template file.
#[derive(Debug, Clone, Default)]
pub struct ExtractedPageContent {
    pub content: String,
    pub leading_include: Option<String>,
    pub trailing_include: Option<String>,
}

/// Canvas-safe fragments from a split-document header ref.
#[derive(Debug, Clone, Default)]
pub struct HeaderFragment {
    pub head_html: String,
    pub body_html: String,
}

/// Load route reference VNodes, classifying header/footer by filename.
pub async fn load_route_ref_parts(
    db: &DatabaseConnection,
    store: &DynFilestore,
    route_id: i64,
) -> Result<RouteRefParts, Box<dyn std::error::Error + Send + Sync>> {
    let mut parts = RouteRefParts::default();
    let refs = RouteRefEntity::find()
        .filter(route_reference::Column::DbRouteId.eq(route_id))
        .all(db)
        .await?;

    for r in refs {
        let Some(vnode) = node::get_by_id(db, r.v_node_id).await? else {
            continue;
        };
        let path = node::get_path(db, &vnode).await;
        let rel = path.trim_start_matches('/').to_string();
        if rel.is_empty() {
            continue;
        }
        let Ok(src) = read_vnode_text(store, &vnode).await else {
            continue;
        };
        let name_lower = vnode.name.to_ascii_lowercase();
        if name_lower.contains("header") {
            parts.header_path = Some(rel);
            parts.header_src = Some(src);
        } else if name_lower.contains("footer") {
            parts.footer_path = Some(rel);
            parts.footer_src = Some(src);
        }
    }
    Ok(parts)
}

/// Parse a minijinja include line: `{% include "path" %}`.
fn parse_include_line(line: &str) -> Option<String> {
    let line = line.trim();
    let rest = line.strip_prefix("{% include ")?.strip_suffix("%}")?.trim();
    if let Some(inner) = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return Some(inner.to_string());
    }
    if let Some(inner) = rest.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        return Some(inner.to_string());
    }
    None
}

/// Strip leading/trailing `{% include %}` lines; return middle content.
pub fn extract_page_content(src: &str) -> ExtractedPageContent {
    let lines: Vec<&str> = src.lines().collect();
    if lines.is_empty() {
        return ExtractedPageContent {
            content: src.to_string(),
            ..Default::default()
        };
    }

    let mut start = 0usize;
    let mut leading = None;
    while start < lines.len() {
        if lines[start].trim().is_empty() {
            start += 1;
            continue;
        }
        if let Some(path) = parse_include_line(lines[start]) {
            if leading.is_none() {
                leading = Some(path);
            }
            start += 1;
            continue;
        }
        break;
    }

    let mut end = lines.len();
    let mut trailing = None;
    while end > start {
        if lines[end - 1].trim().is_empty() {
            end -= 1;
            continue;
        }
        if let Some(path) = parse_include_line(lines[end - 1]) {
            trailing = Some(path);
            end -= 1;
            continue;
        }
        break;
    }

    let content = if start >= end {
        src.trim().to_string()
    } else {
        lines[start..end].join("\n")
    };

    ExtractedPageContent {
        content,
        leading_include: leading,
        trailing_include: trailing,
    }
}

/// Extract canvas `<head>` assets and body fragment from a split-document header ref.
pub fn builder_header_fragment(src: &str) -> HeaderFragment {
    let lower = src.to_lowercase();

    let head_html = extract_tag_inner(src, &lower, "<head", "</head>");

    let body_html = if let Some(body_start) = lower.find("<body") {
        let after_open = lower[body_start..].find('>').map(|i| body_start + i + 1);
        if let Some(content_start) = after_open {
            let body_part = &src[content_start..];
            let body_lower = body_part.to_lowercase();
            if let Some(close) = body_lower.rfind("</body>") {
                body_part[..close].trim().to_string()
            } else {
                body_part.trim().to_string()
            }
        } else {
            src.trim().to_string()
        }
    } else {
        src.trim().to_string()
    };

    HeaderFragment {
        head_html,
        body_html,
    }
}

/// Return footer partial with trailing document closers stripped for canvas use.
pub fn builder_footer_fragment(src: &str) -> String {
    let mut s = src.trim().to_string();
    let lower = s.to_lowercase();
    if let Some(idx) = lower.rfind("</body>") {
        s = s[..idx].trim_end().to_string();
    }
    let lower = s.to_lowercase();
    if let Some(idx) = lower.rfind("</html>") {
        s = s[..idx].trim_end().to_string();
    }
    s
}

fn extract_tag_inner(src: &str, lower: &str, open_tag: &str, close_tag: &str) -> String {
    let Some(tag_start) = lower.find(open_tag) else {
        return String::new();
    };
    let Some(after_open) = lower[tag_start..].find('>').map(|i| tag_start + i + 1) else {
        return String::new();
    };
    let Some(close_start) = lower[after_open..].find(close_tag).map(|i| after_open + i) else {
        return String::new();
    };
    src[after_open..close_start].trim().to_string()
}

/// Rebuild a page template file with include directives and editable middle content.
pub fn compose_page_template(
    header_inc: Option<&str>,
    content: &str,
    footer_inc: Option<&str>,
) -> String {
    let mut out = String::new();
    if let Some(h) = header_inc {
        out.push_str(&format!("{{% include \"{h}\" %}}"));
        out.push_str("\n\n");
    }
    out.push_str(content.trim());
    if let Some(f) = footer_inc {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
        out.push_str(&format!("{{% include \"{f}\" %}}"));
    }
    out
}

/// Merge GrapesJS canvas CSS into content HTML as a leading `<style>` block.
pub fn merge_content_css(content: &str, css: &str) -> String {
    let css = css.trim();
    if css.is_empty() {
        return content.trim().to_string();
    }
    format!("<style>\n{css}\n</style>\n{}", content.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_page_content_strips_includes() {
        let src = r#"{% include "website/header.html" %}

<h1>Hello</h1>

{% include "website/footer.html" %}
"#;
        let extracted = extract_page_content(src);
        assert_eq!(extracted.leading_include.as_deref(), Some("website/header.html"));
        assert_eq!(extracted.trailing_include.as_deref(), Some("website/footer.html"));
        assert!(extracted.content.contains("<h1>Hello</h1>"));
        assert!(!extracted.content.contains("include"));
    }

    #[test]
    fn extract_page_content_no_includes() {
        let src = "<h1>Flat page</h1>";
        let extracted = extract_page_content(src);
        assert_eq!(extracted.content, src);
        assert!(extracted.leading_include.is_none());
    }

    #[test]
    fn compose_page_template_roundtrip() {
        let composed = compose_page_template(
            Some("website/header.html"),
            "<h1>Body</h1>",
            Some("website/footer.html"),
        );
        let extracted = extract_page_content(&composed);
        assert_eq!(extracted.content.trim(), "<h1>Body</h1>");
    }

    #[test]
    fn builder_footer_fragment_strips_closers() {
        let src = "<footer>Foot</footer>\n</body>\n</html>\n";
        assert_eq!(builder_footer_fragment(src), "<footer>Foot</footer>");
    }

    #[test]
    fn builder_header_fragment_splits_head_and_body() {
        let src = "<!DOCTYPE html><html><head><style>x{}</style></head><body><nav>N</nav></body></html>";
        let frag = builder_header_fragment(src);
        assert!(frag.head_html.contains("style"));
        assert!(frag.body_html.contains("<nav>N</nav>"));
    }
}
