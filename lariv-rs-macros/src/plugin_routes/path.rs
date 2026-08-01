//! Parse Axum-style path literals into static segments and typed params.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    Static(String),
    Param { name: String, splat: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPath {
    pub segments: Vec<PathSegment>,
    pub raw: String,
}

/// Split `"/users/u/{id}/edit"` into static and `{param}` / `{*param}` segments.
pub fn parse_path(path: &str) -> ParsedPath {
    let mut segments = Vec::new();
    let mut rest = path;

    while let Some(open) = rest.find('{') {
        if open > 0 {
            segments.push(PathSegment::Static(rest[..open].to_string()));
        }
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            break;
        };
        let inner = &after_open[..close];
        let splat = inner.starts_with('*');
        let name = if splat { &inner[1..] } else { inner }.to_string();
        segments.push(PathSegment::Param { name, splat });
        rest = &after_open[close + 1..];
    }
    if !rest.is_empty() {
        segments.push(PathSegment::Static(rest.to_string()));
    }

    ParsedPath {
        segments,
        raw: path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_id_param() {
        let p = parse_path("/users/u/{id}");
        assert_eq!(
            p.segments,
            vec![
                PathSegment::Static("/users/u/".into()),
                PathSegment::Param {
                    name: "id".into(),
                    splat: false
                },
            ]
        );
    }

    #[test]
    fn parse_parent_id_and_splat() {
        let p = parse_path("/filesystem/browse/{parent_id}");
        assert!(matches!(
            p.segments.last(),
            Some(PathSegment::Param { name, splat: false }) if name == "parent_id"
        ));

        let p = parse_path("/static/pwa/{*path}");
        assert!(matches!(
            p.segments.last(),
            Some(PathSegment::Param { name, splat: true }) if name == "path"
        ));
    }
}
