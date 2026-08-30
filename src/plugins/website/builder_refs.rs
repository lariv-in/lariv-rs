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

/// Pull `<style>` blocks out of page content so GrapesJS cannot strip/mangle them.
///
/// GrapesJS always extracts `<style>` tags into its CSSOM parser (which drops modern
/// CSS) and removes them from the component tree. Returning raw CSS separately lets
/// the builder re-inject and re-save it, similar to theme JS surviving outside GrapesJS.
pub fn split_content_styles(content: &str) -> (String, String) {
    let mut html = String::with_capacity(content.len());
    let mut css_parts: Vec<&str> = Vec::new();
    let lower = content.to_lowercase();
    let mut search = 0usize;

    while let Some(rel) = lower[search..].find("<style") {
        let start = search + rel;
        html.push_str(&content[search..start]);

        let after_open = start + 6;
        let Some(gt) = lower[after_open..].find('>').map(|i| after_open + i) else {
            html.push_str(&content[start..]);
            search = content.len();
            break;
        };
        let inner_start = gt + 1;
        let Some(close_rel) = lower[inner_start..].find("</style>") else {
            html.push_str(&content[start..]);
            search = content.len();
            break;
        };
        let inner_end = inner_start + close_rel;
        let mut end = inner_end + "</style>".len();
        if content.as_bytes().get(end) == Some(&b'\n') {
            end += 1;
        }

        let css = content[inner_start..inner_end].trim();
        if !css.is_empty() {
            css_parts.push(css);
        }
        search = end;
    }
    if search < content.len() {
        html.push_str(&content[search..]);
    }

    let css = css_parts.join("\n\n");
    (html.trim().to_string(), css)
}

/// Merge GrapesJS canvas CSS into content HTML as a leading `<style>` block.
///
/// Existing `<style>` tags in `content` are replaced (not stacked) so save round-trips
/// stay stable when the client already sent CSS separately.
pub fn merge_content_css(content: &str, css: &str) -> String {
    let (body, existing_css) = split_content_styles(content);
    let css = css.trim();
    let merged = if css.is_empty() {
        existing_css
    } else {
        css.to_string()
    };
    let merged = merged.trim();
    if merged.is_empty() {
        return body;
    }
    format!("<style>\n{merged}\n</style>\n{body}")
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
        assert_eq!(
            extracted.leading_include.as_deref(),
            Some("website/header.html")
        );
        assert_eq!(
            extracted.trailing_include.as_deref(),
            Some("website/footer.html")
        );
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
        let src =
            "<!DOCTYPE html><html><head><style>x{}</style></head><body><nav>N</nav></body></html>";
        let frag = builder_header_fragment(src);
        assert!(frag.head_html.contains("style"));
        assert!(frag.body_html.contains("<nav>N</nav>"));
    }

    #[test]
    fn split_content_styles_extracts_and_removes_style_blocks() {
        let src = r#"<style>
#machines .machine-photo { max-height: 9.5rem; }
@media (max-width: 720px) {
  #machines .machine-photo { max-height: 11rem; }
}
</style>
<section id="machines"><h2>Machines</h2></section>
"#;
        let (html, css) = split_content_styles(src);
        assert!(!html.contains("<style"));
        assert!(html.contains(r#"<section id="machines">"#));
        assert!(css.contains("#machines .machine-photo"));
        assert!(css.contains("@media (max-width: 720px)"));
    }

    #[test]
    fn merge_content_css_roundtrips_without_stacking() {
        let body = "<h1>Hi</h1>";
        let once = merge_content_css(body, "h1 { color: red; }");
        let twice = merge_content_css(&once, "h1 { color: blue; }");
        assert_eq!(twice.matches("<style").count(), 1);
        assert!(twice.contains("color: blue"));
        assert!(!twice.contains("color: red"));
    }
}
