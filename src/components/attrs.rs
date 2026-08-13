//! HTML attribute maps for HTMX and extra element attributes.

use std::collections::BTreeMap;

use maud::{Markup, PreEscaped};

/// Ordered map of HTML attribute names to values.
#[derive(Clone, Debug, Default)]
pub struct HtmlAttrs {
    pub attrs: BTreeMap<String, String>,
}

impl HtmlAttrs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attrs.insert(key.into(), value.into());
        self
    }

    pub fn merge(mut self, other: &HtmlAttrs) -> Self {
        for (k, v) in &other.attrs {
            self.attrs.insert(k.clone(), v.clone());
        }
        self
    }

    pub fn is_empty(&self) -> bool {
        self.attrs.is_empty()
    }

    /// Space-prefixed `key="escaped"` pairs for injection into open tags.
    pub fn as_string(&self) -> String {
        if self.attrs.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        for (k, v) in &self.attrs {
            out.push(' ');
            out.push_str(k);
            out.push_str("=\"");
            for ch in v.chars() {
                match ch {
                    '&' => out.push_str("&amp;"),
                    '"' => out.push_str("&quot;"),
                    '<' => out.push_str("&lt;"),
                    _ => out.push(ch),
                }
            }
            out.push('"');
        }
        out
    }

    /// Render as a leading-space attribute string suitable for `PreEscaped` injection.
    pub fn render(&self) -> Markup {
        PreEscaped(self.as_string())
    }
}

impl FromIterator<(String, String)> for HtmlAttrs {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self {
            attrs: iter.into_iter().collect(),
        }
    }
}

/// Escape a value for use inside a double-quoted HTML attribute.
pub fn escape_attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            _ => out.push(ch),
        }
    }
    out
}

/// True when `Alpine.$data` appears as an HTML text node (an attribute's quotes broke).
#[cfg(test)]
pub(crate) fn alpine_js_leaked_as_text(html: &str) -> bool {
    let mut in_tag = false;
    let mut in_quote: Option<char> = None;
    let mut text = String::new();
    for c in html.chars() {
        if !in_tag {
            if c == '<' {
                if text.contains("Alpine.$data") {
                    return true;
                }
                text.clear();
                in_tag = true;
            } else {
                text.push(c);
            }
        } else if let Some(q) = in_quote {
            if c == q {
                in_quote = None;
            }
        } else if c == '"' || c == '\'' {
            in_quote = Some(c);
        } else if c == '>' {
            in_tag = false;
        }
    }
    text.contains("Alpine.$data")
}
