use std::collections::HashMap;

use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};

use crate::render::escape_html;

/// Renders Markdown to HTML (CommonMark plus tables, footnotes, strikethrough).
///
/// Raw HTML in the source passes through verbatim — content is trusted,
/// authored input, consistent with `GuideSection::body_html`.
///
/// Headings get a slug `id` derived from their text, and their text is wrapped
/// in a self-link (`<a class="cb-hlink" href="#slug">`), so every title is
/// perma-linkable. Duplicate slugs within one document get `-2`, `-3`, …
pub fn render_markdown(markdown: &str) -> String {
    let options = pulldown_cmark::Options::ENABLE_TABLES
        | pulldown_cmark::Options::ENABLE_FOOTNOTES
        | pulldown_cmark::Options::ENABLE_STRIKETHROUGH;
    let parser = pulldown_cmark::Parser::new_ext(markdown, options);
    let events = render_mermaid_blocks(open_external_links_in_new_tab(anchor_headings(parser)));
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events.into_iter());
    html
}

/// Rewrites every heading as `<hN id="slug"><a class="cb-hlink" href="#slug">…</a></hN>`.
fn anchor_headings<'a>(parser: impl Iterator<Item = Event<'a>>) -> Vec<Event<'a>> {
    let mut out = Vec::new();
    let mut buffer: Option<(pulldown_cmark::HeadingLevel, Vec<Event<'a>>)> = None;
    let mut seen: HashMap<String, u32> = HashMap::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                buffer = Some((level, Vec::new()));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, inner)) = buffer.take() {
                    let text: String = inner
                        .iter()
                        .filter_map(|e| match e {
                            Event::Text(t) | Event::Code(t) => Some(t.as_ref()),
                            _ => None,
                        })
                        .collect();
                    let slug = unique_slug(&slugify(&text), &mut seen);
                    out.push(Event::Html(
                        format!("<{level} id=\"{slug}\"><a class=\"cb-hlink\" href=\"#{slug}\">")
                            .into(),
                    ));
                    out.extend(inner);
                    out.push(Event::Html(format!("</a></{level}>").into()));
                }
            }
            other => match &mut buffer {
                Some((_, inner)) => inner.push(other),
                None => out.push(other),
            },
        }
    }
    out
}

/// Rewrites links to external sites (`http://` / `https://`) so they open in a
/// new tab, with `rel="noopener noreferrer"` to sever the opener reference.
///
/// Relative links (course pages `/…`, heading anchors `#…`) and the heading
/// self-links — which are already raw `Event::Html` — are left untouched, so
/// in-site navigation stays in the same tab.
fn open_external_links_in_new_tab(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut out = Vec::with_capacity(events.len());
    for event in events {
        match event {
            Event::Start(Tag::Link {
                dest_url, title, ..
            }) if is_external(&dest_url) => {
                let mut tag = format!("<a href=\"{}\"", escape_html(&dest_url));
                if !title.is_empty() {
                    tag.push_str(&format!(" title=\"{}\"", escape_html(&title)));
                }
                tag.push_str(" target=\"_blank\" rel=\"noopener noreferrer\">");
                // `TagEnd::Link` still renders as `</a>`, closing this tag.
                out.push(Event::Html(tag.into()));
            }
            other => out.push(other),
        }
    }
    out
}

/// Rewrites fenced ```` ```mermaid ```` blocks into `<pre class="mermaid">…</pre>`,
/// the element mermaid.js renders. The diagram source is HTML-escaped so the
/// markup is valid; the browser decodes it back as `textContent` for mermaid.
///
/// Any other fenced or indented code block is left untouched (rendered by
/// pulldown-cmark as `<pre><code>` as usual).
fn render_mermaid_blocks(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut out = Vec::with_capacity(events.len());
    let mut source: Option<String> = None;
    for event in events {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info)))
                if info.as_ref() == "mermaid" =>
            {
                source = Some(String::new());
            }
            Event::Text(text) if source.is_some() => {
                if let Some(buffer) = source.as_mut() {
                    buffer.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) if source.is_some() => {
                let diagram = source.take().unwrap_or_default();
                out.push(Event::Html(
                    format!("<pre class=\"mermaid\">{}</pre>", escape_html(&diagram)).into(),
                ));
            }
            other => out.push(other),
        }
    }
    out
}

/// True for absolute `http`/`https` URLs — links that leave the site.
fn is_external(dest_url: &str) -> bool {
    dest_url.starts_with("http://") || dest_url.starts_with("https://")
}

/// Lowercases, strips Spanish diacritics, and maps runs of non-alphanumerics
/// to single hyphens. Empty input slugs to `"titulo"`.
pub(crate) fn slugify(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());
    for c in text.chars() {
        let c = match c {
            'á' | 'Á' => 'a',
            'é' | 'É' => 'e',
            'í' | 'Í' => 'i',
            'ó' | 'Ó' => 'o',
            'ú' | 'Ú' | 'ü' | 'Ü' => 'u',
            'ñ' | 'Ñ' => 'n',
            _ => c,
        };
        if c.is_alphanumeric() {
            for lower in c.to_lowercase() {
                slug.push(lower);
            }
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-').to_owned();
    if slug.is_empty() {
        "titulo".to_owned()
    } else {
        slug
    }
}

/// Appends `-2`, `-3`, … on repeated slugs within one document.
fn unique_slug(slug: &str, seen: &mut HashMap<String, u32>) -> String {
    let count = seen.entry(slug.to_owned()).or_insert(0);
    *count += 1;
    if *count == 1 {
        slug.to_owned()
    } else {
        format!("{slug}-{count}")
    }
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
        assert!(html.contains("<h2"));
        assert!(html.contains("Título"));
    }

    #[test]
    fn fenced_code_block_renders_pre_code() {
        let html = render_markdown("```rust\nfn main() {}\n```\n");
        assert!(html.contains("<pre>"));
        assert!(html.contains("<code"));
    }

    #[test]
    fn mermaid_fence_renders_pre_mermaid() {
        let html = render_markdown("```mermaid\nflowchart LR\n  A --> B\n```\n");
        assert!(html.contains("<pre class=\"mermaid\">"));
        // The arrow is HTML-escaped inside the <pre>, not rendered as a code block.
        assert!(html.contains("A --&gt; B"));
        assert!(!html.contains("<code"));
    }

    #[test]
    fn non_mermaid_fence_still_renders_code_block() {
        let html = render_markdown("```yaml\nkey: value\n```\n");
        assert!(html.contains("<pre>"));
        assert!(html.contains("<code"));
        assert!(!html.contains("class=\"mermaid\""));
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

    // ── heading anchors ──────────────────────────────────────────────────

    #[test]
    fn heading_gets_slug_id_and_self_link() {
        let html = render_markdown("### Crear el repositorio\n");
        assert!(html.contains(
            "<h3 id=\"crear-el-repositorio\">\
             <a class=\"cb-hlink\" href=\"#crear-el-repositorio\">Crear el repositorio</a></h3>"
        ));
    }

    #[test]
    fn heading_slug_strips_accents_and_punctuation() {
        let html = render_markdown("## Práctica guiada: crear el repositorio\n");
        assert!(html.contains("id=\"practica-guiada-crear-el-repositorio\""));
    }

    #[test]
    fn heading_with_inline_code_keeps_markup_inside_link() {
        let html = render_markdown("### La rama `desarrollo`\n");
        assert!(html.contains("id=\"la-rama-desarrollo\""));
        assert!(html.contains("<code>desarrollo</code></a>"));
    }

    #[test]
    fn duplicate_headings_get_numbered_slugs() {
        let html = render_markdown("## Pasos\n\ntexto\n\n## Pasos\n");
        assert!(html.contains("id=\"pasos\""));
        assert!(html.contains("id=\"pasos-2\""));
    }

    #[test]
    fn slugify_handles_spanish_characters() {
        assert_eq!(slugify("Configuración añadida"), "configuracion-anadida");
    }

    #[test]
    fn slugify_collapses_separator_runs() {
        assert_eq!(
            slugify("Ejercicio 1 — Cree su repositorio"),
            "ejercicio-1-cree-su-repositorio"
        );
    }

    #[test]
    fn slugify_empty_falls_back() {
        assert_eq!(slugify("—"), "titulo");
    }

    // ── external links ───────────────────────────────────────────────────

    #[test]
    fn external_https_link_opens_in_new_tab() {
        let html = render_markdown("[AWS](https://aws.amazon.com/blogs/).\n");
        assert!(html.contains(
            "<a href=\"https://aws.amazon.com/blogs/\" \
             target=\"_blank\" rel=\"noopener noreferrer\">AWS</a>"
        ));
    }

    #[test]
    fn external_http_link_opens_in_new_tab() {
        let html = render_markdown("[x](http://example.com)\n");
        assert!(html.contains("target=\"_blank\""));
        assert!(html.contains("rel=\"noopener noreferrer\""));
    }

    #[test]
    fn external_link_title_is_preserved_and_escaped() {
        let html = render_markdown("[x](https://example.com \"Más <info>\")\n");
        assert!(html.contains("title=\"Más &lt;info&gt;\""));
        assert!(html.contains("target=\"_blank\""));
    }

    #[test]
    fn relative_link_stays_in_same_tab() {
        let html = render_markdown("[Sección](/courses/aws-devops/x)\n");
        assert!(html.contains("<a href=\"/courses/aws-devops/x\">Sección</a>"));
        assert!(!html.contains("target=\"_blank\""));
    }

    #[test]
    fn anchor_link_stays_in_same_tab() {
        let html = render_markdown("[Arriba](#top)\n");
        assert!(html.contains("<a href=\"#top\">Arriba</a>"));
        assert!(!html.contains("target=\"_blank\""));
    }

    #[test]
    fn heading_self_link_is_not_marked_external() {
        let html = render_markdown("## Título\n");
        assert!(!html.contains("target=\"_blank\""));
    }
}
