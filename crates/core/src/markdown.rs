/// Renders Markdown to HTML (CommonMark plus tables, footnotes, strikethrough).
///
/// Raw HTML in the source passes through verbatim — content is trusted,
/// authored input, consistent with `GuideSection::body_html`.
pub fn render_markdown(markdown: &str) -> String {
    let options = pulldown_cmark::Options::ENABLE_TABLES
        | pulldown_cmark::Options::ENABLE_FOOTNOTES
        | pulldown_cmark::Options::ENABLE_STRIKETHROUGH;
    let parser = pulldown_cmark::Parser::new_ext(markdown, options);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn paragraph_renders_as_p_element() {
        let html = render_markdown("Hello world.\n");
        assert!(html.contains("<p>Hello world.</p>"));
    }

    #[test]
    fn heading_renders_as_h2() {
        let html = render_markdown("## Título\n");
        assert!(html.contains("<h2>"));
        assert!(html.contains("Título"));
    }

    #[test]
    fn fenced_code_block_renders_pre_code() {
        let html = render_markdown("```rust\nfn main() {}\n```\n");
        assert!(html.contains("<pre>"));
        assert!(html.contains("<code"));
    }

    #[test]
    fn table_renders_table_element() {
        let html = render_markdown("| A | B |\n|---|---|\n| 1 | 2 |\n");
        assert!(html.contains("<table>"));
    }

    #[test]
    fn raw_inline_html_passes_through() {
        let html = render_markdown("<span class=\"x\">text</span>\n");
        assert!(html.contains("<span class=\"x\">text</span>"));
    }

    #[test]
    fn empty_input_renders_empty() {
        assert_eq!(render_markdown(""), "");
    }
}
