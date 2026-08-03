//! Blank page starter and editable-name helpers.

pub const BLANK_PAGE_STARTER_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>New page</title>
</head>
<body>
  <h1>New page</h1>
</body>
</html>
"#;

/// Whether a VNode name is editable in the GrapesJS builder.
pub fn is_editable_html_name(name: &str) -> bool {
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(ext.as_str(), "html" | "htm" | "tmpl")
}
