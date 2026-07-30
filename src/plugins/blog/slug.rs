//! Port of Go `getters.TitleToFormSlug`.

/// Derive a URL path segment from a title (lowercase, hyphen-separated,
/// ASCII letters and digits only; max 160 chars). Empty input becomes `"form"`.
pub fn title_to_form_slug(title: &str) -> String {
    let title = title.trim().to_lowercase();
    let mut out = String::new();
    let mut last_hyphen = false;
    for r in title.chars() {
        match r {
            'a'..='z' | '0'..='9' => {
                out.push(r);
                last_hyphen = false;
            }
            ' ' | '-' | '.' | '_' => {
                if !out.is_empty() && !last_hyphen {
                    out.push('-');
                    last_hyphen = true;
                }
            }
            _ => {}
        }
    }
    let s = out.trim_matches('-').to_string();
    if s.is_empty() {
        return "form".into();
    }
    let runes: Vec<char> = s.chars().collect();
    if runes.len() > 160 {
        let truncated: String = runes[..160].iter().collect();
        let s = truncated.trim_end_matches('-').to_string();
        if s.is_empty() {
            return "form".into();
        }
        return s;
    }
    s
}

/// Canonicalize a blog slug: blank derives from title (or `"blog"`), else slugify.
pub fn resolve_blog_slug(title: &str, slug: &str) -> String {
    let slug = slug.trim();
    if slug.is_empty() {
        let title = title.trim();
        if title.is_empty() {
            "blog".into()
        } else {
            title_to_form_slug(title)
        }
    } else {
        title_to_form_slug(slug)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_from_title() {
        assert_eq!(title_to_form_slug("Hello World"), "hello-world");
        assert_eq!(title_to_form_slug(""), "form");
        assert_eq!(resolve_blog_slug("My Post", ""), "my-post");
        assert_eq!(resolve_blog_slug("", ""), "blog");
        assert_eq!(resolve_blog_slug("X", "Custom Slug!"), "custom-slug");
    }
}
