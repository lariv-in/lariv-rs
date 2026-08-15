//! Markdown → HTML with Tailwind / daisyUI utility classes.
//!
//! Tailwind Preflight removes browser defaults, so bare `<h1>` / `<ul>` tags
//! render unstyled. Class names are emitted on each tag so `@tailwindcss/browser`
//! can pick them up from the DOM.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::components::attrs::escape_attr;

const P: &str = "my-2 max-w-full break-words text-base-content first:mt-0";
const UL: &str = "my-2 max-w-full list-disc space-y-1 pl-6 first:mt-0";
const OL: &str = "my-2 max-w-full list-decimal space-y-1 pl-6 first:mt-0";
const LI: &str = "my-0.5 max-w-full break-words";
const A: &str = "link link-primary break-all";
const CODE: &str = "whitespace-pre-wrap break-all rounded bg-base-200 px-1 py-0.5 font-mono text-sm";
const PRE: &str =
    "my-2 max-w-full whitespace-pre-wrap break-words rounded-md bg-base-200 p-3 font-mono text-sm first:mt-0";
const BLOCKQUOTE: &str =
    "my-2 max-w-full break-words border-l-4 border-base-300 pl-4 italic text-base-content/80 first:mt-0";
const HR: &str = "my-4 border-base-300";
const TABLE: &str = "table table-zebra table-sm my-2 w-full max-w-full first:mt-0";
const TH: &str = "text-left font-semibold break-words";
const IMG: &str = "my-2 max-w-full rounded-md";
const STRONG: &str = "font-bold";
const EM: &str = "italic";
const STRIKE: &str = "line-through";
const CHECKBOX: &str = "checkbox checkbox-sm mr-2 align-middle";

fn heading_class(level: HeadingLevel) -> &'static str {
    match level {
        HeadingLevel::H1 => "mt-6 mb-2 text-2xl font-bold text-base-content first:mt-0",
        HeadingLevel::H2 => "mt-5 mb-2 text-xl font-semibold text-base-content first:mt-0",
        HeadingLevel::H3 => "mt-4 mb-2 text-lg font-semibold text-base-content first:mt-0",
        HeadingLevel::H4 => "mt-3 mb-1 text-base font-semibold text-base-content first:mt-0",
        HeadingLevel::H5 | HeadingLevel::H6 => {
            "mt-3 mb-1 text-sm font-semibold text-base-content first:mt-0"
        }
    }
}

/// Parse markdown to HTML, tagging each element with Tailwind utility classes.
pub fn render_markdown(md: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(md, options);
    let mut html_out = String::new();
    push_markdown_html(&mut html_out, parser);
    html_out
}

fn push_markdown_html<'a, I>(out: &mut String, parser: I)
where
    I: Iterator<Item = Event<'a>>,
{
    let mut in_table_head = false;
    let mut in_image = false;
    let mut image_src = String::new();
    let mut image_title = String::new();
    let mut image_alt = String::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => push_open(out, "p", P),
                Tag::Heading {
                    level,
                    id,
                    classes,
                    attrs: _,
                } => {
                    let mut class = heading_class(level).to_string();
                    for extra in classes {
                        class.push(' ');
                        class.push_str(&extra);
                    }
                    out.push('<');
                    out.push_str(level.to_string().as_str());
                    out.push_str(" class=\"");
                    out.push_str(&escape_attr(&class));
                    out.push('"');
                    if let Some(id) = id {
                        out.push_str(" id=\"");
                        out.push_str(&escape_attr(&id));
                        out.push('"');
                    }
                    out.push('>');
                }
                Tag::BlockQuote(_) => push_open(out, "blockquote", BLOCKQUOTE),
                Tag::CodeBlock(_) => {
                    push_open(out, "pre", PRE);
                    out.push_str("<code>");
                }
                Tag::List(None) => push_open(out, "ul", UL),
                Tag::List(Some(start)) => {
                    out.push_str("<ol class=\"");
                    out.push_str(OL);
                    out.push('"');
                    if start != 1 {
                        out.push_str(" start=\"");
                        out.push_str(&start.to_string());
                        out.push('"');
                    }
                    out.push('>');
                }
                Tag::Item => push_open(out, "li", LI),
                Tag::Table(_) => push_open(out, "table", TABLE),
                Tag::TableHead => {
                    in_table_head = true;
                    out.push_str("<thead><tr>");
                }
                Tag::TableRow => {
                    if !in_table_head {
                        out.push_str("<tr>");
                    }
                }
                Tag::TableCell => {
                    if in_table_head {
                        push_open(out, "th", TH);
                    } else {
                        out.push_str("<td>");
                    }
                }
                Tag::Emphasis => push_open(out, "em", EM),
                Tag::Strong => push_open(out, "strong", STRONG),
                Tag::Strikethrough => push_open(out, "del", STRIKE),
                Tag::Link {
                    dest_url, title, ..
                } => {
                    out.push_str("<a class=\"");
                    out.push_str(A);
                    out.push_str("\" href=\"");
                    out.push_str(&escape_attr(&dest_url));
                    out.push('"');
                    if !title.is_empty() {
                        out.push_str(" title=\"");
                        out.push_str(&escape_attr(&title));
                        out.push('"');
                    }
                    if dest_url.starts_with("http://") || dest_url.starts_with("https://") {
                        out.push_str(" target=\"_blank\" rel=\"noopener noreferrer\"");
                    }
                    out.push('>');
                }
                Tag::Image {
                    dest_url, title, ..
                } => {
                    in_image = true;
                    image_src = dest_url.into_string();
                    image_title = title.into_string();
                    image_alt.clear();
                }
                Tag::HtmlBlock
                | Tag::FootnoteDefinition(_)
                | Tag::MetadataBlock(_)
                | Tag::DefinitionList
                | Tag::DefinitionListTitle
                | Tag::DefinitionListDefinition
                | Tag::Superscript
                | Tag::Subscript => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => out.push_str("</p>"),
                TagEnd::Heading(level) => {
                    out.push_str("</");
                    out.push_str(&level.to_string());
                    out.push('>');
                }
                TagEnd::BlockQuote(_) => out.push_str("</blockquote>"),
                TagEnd::CodeBlock => out.push_str("</code></pre>"),
                TagEnd::List(false) => out.push_str("</ul>"),
                TagEnd::List(true) => out.push_str("</ol>"),
                TagEnd::Item => out.push_str("</li>"),
                TagEnd::Table => out.push_str("</tbody></table>"),
                TagEnd::TableHead => {
                    in_table_head = false;
                    out.push_str("</tr></thead><tbody>");
                }
                TagEnd::TableRow => {
                    if !in_table_head {
                        out.push_str("</tr>");
                    }
                }
                TagEnd::TableCell => {
                    if in_table_head {
                        out.push_str("</th>");
                    } else {
                        out.push_str("</td>");
                    }
                }
                TagEnd::Emphasis => out.push_str("</em>"),
                TagEnd::Strong => out.push_str("</strong>"),
                TagEnd::Strikethrough => out.push_str("</del>"),
                TagEnd::Link => out.push_str("</a>"),
                TagEnd::Image => {
                    out.push_str("<img class=\"");
                    out.push_str(IMG);
                    out.push_str("\" src=\"");
                    out.push_str(&escape_attr(&image_src));
                    out.push_str("\" alt=\"");
                    out.push_str(&escape_attr(&image_alt));
                    out.push('"');
                    if !image_title.is_empty() {
                        out.push_str(" title=\"");
                        out.push_str(&escape_attr(&image_title));
                        out.push('"');
                    }
                    out.push('>');
                    in_image = false;
                    image_src.clear();
                    image_title.clear();
                    image_alt.clear();
                }
                TagEnd::HtmlBlock
                | TagEnd::FootnoteDefinition
                | TagEnd::MetadataBlock(_)
                | TagEnd::DefinitionList
                | TagEnd::DefinitionListTitle
                | TagEnd::DefinitionListDefinition
                | TagEnd::Superscript
                | TagEnd::Subscript => {}
            },
            Event::Text(text) => {
                if in_image {
                    image_alt.push_str(&text);
                } else {
                    push_escaped(out, &text);
                }
            }
            Event::Code(text) => {
                if in_image {
                    image_alt.push_str(&text);
                } else {
                    push_open(out, "code", CODE);
                    push_escaped(out, &text);
                    out.push_str("</code>");
                }
            }
            Event::Html(html) | Event::InlineHtml(html) => push_escaped(out, &html),
            Event::SoftBreak => out.push('\n'),
            Event::HardBreak => out.push_str("<br>"),
            Event::Rule => {
                out.push_str("<hr class=\"");
                out.push_str(HR);
                out.push_str("\">");
            }
            Event::TaskListMarker(checked) => {
                out.push_str("<input type=\"checkbox\" disabled class=\"");
                out.push_str(CHECKBOX);
                out.push('"');
                if checked {
                    out.push_str(" checked");
                }
                out.push('>');
            }
            Event::FootnoteReference(_) | Event::InlineMath(_) | Event::DisplayMath(_) => {}
        }
    }
}

fn push_open(out: &mut String, tag: &str, class: &str) {
    out.push('<');
    out.push_str(tag);
    out.push_str(" class=\"");
    out.push_str(class);
    out.push_str("\">");
}

fn push_escaped(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_and_lists_use_tailwind_classes() {
        let html = render_markdown("# Title\n\nA paragraph.\n\n- item\n\n1. one\n");
        assert!(html.contains("text-2xl font-bold"), "{html}");
        assert!(html.contains("list-disc"), "{html}");
        assert!(html.contains("list-decimal"), "{html}");
        assert!(html.contains("<p class=\"my-2"), "{html}");
    }

    #[test]
    fn inline_code_and_links_are_styled() {
        let html = render_markdown("See [`code`](https://example.com) here.");
        assert!(html.contains("font-mono"), "{html}");
        assert!(html.contains("link link-primary"), "{html}");
        assert!(html.contains("href=\"https://example.com\""), "{html}");
        assert!(html.contains("target=\"_blank\""), "{html}");
    }

    #[test]
    fn tables_use_daisyui_table_classes() {
        let html = render_markdown("| a | b |\n| --- | --- |\n| 1 | 2 |\n");
        assert!(html.contains("table table-zebra"), "{html}");
        assert!(html.contains("<th class=\""), "{html}");
    }

    #[test]
    fn html_in_markdown_is_escaped() {
        let html = render_markdown("Hello <script>alert(1)</script>");
        assert!(html.contains("&lt;script&gt;"), "{html}");
        assert!(!html.contains("<script>"), "{html}");
    }
}
