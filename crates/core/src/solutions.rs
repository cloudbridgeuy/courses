use crate::error::{Error, Result};
use crate::markdown::render_markdown;
use crate::render::escape_html;
use crate::slides::{extract_anchors, render_slide_content};

/// One top-level segment of a section body: plain Markdown, a hidden solution,
/// a slides-only block, an inline slide (rendered in both guide and slides), a
/// title-only slide, an always-visible warning, or a collapsible extra block.
///
/// A directive's payload (`md`) is captured verbatim — it may itself contain
/// nested directives, which the recursive renderers expand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Markdown(String),
    Solution(String),
    Slide { md: String, light: bool },
    InlineSlide { md: String, light: bool },
    TitleSlide { text: String },
    Warning(String),
    Extra { title: String, md: String },
    App(String),
}

/// A rendered slide HTML fragment with its display variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideFragment {
    pub html: String,
    pub light: bool,
}

/// A directive opener recognized at the start of a fenced block.
enum Opener {
    Solution,
    Slide { light: bool },
    InlineSlide { light: bool },
    TitleSlide { text: String },
    Warning,
    Extra { title: String },
    App,
}

/// Recognizes a directive opener on its own line. The slide fences
/// (`:::slide`, `:::inline-slide`, `:::title-slide`) take no space and accept
/// an optional `light` modifier (except title-slide). `::: warning` takes no
/// arguments — with trailing text it is plain Markdown. `::: extra` takes an
/// optional title after the keyword.
fn opener(fence: &str) -> Option<Opener> {
    if fence == "::: solucion" {
        Some(Opener::Solution)
    } else if fence == ":::slide" || fence == ":::slide light" {
        Some(Opener::Slide {
            light: fence == ":::slide light",
        })
    } else if fence == ":::inline-slide" || fence == ":::inline-slide light" {
        Some(Opener::InlineSlide {
            light: fence == ":::inline-slide light",
        })
    } else if let Some(rest) = fence.strip_prefix(":::title-slide") {
        (rest.is_empty() || rest.starts_with(' ')).then(|| Opener::TitleSlide {
            text: rest.trim().to_owned(),
        })
    } else if fence == "::: warning" {
        Some(Opener::Warning)
    } else if let Some(rest) = fence.strip_prefix("::: extra") {
        (rest.is_empty() || rest.starts_with(' ')).then(|| Opener::Extra {
            title: rest.trim().to_owned(),
        })
    } else if fence == ":::app" {
        Some(Opener::App)
    } else {
        None
    }
}

impl Opener {
    fn into_segment(self, md: String) -> Result<Segment> {
        Ok(match self {
            Opener::Solution => Segment::Solution(md),
            Opener::Slide { light } => Segment::Slide { md, light },
            Opener::InlineSlide { light } => Segment::InlineSlide { md, light },
            Opener::TitleSlide { text } => {
                if !md.trim().is_empty() {
                    return Err(Error::TitleSlideNotEmpty);
                }
                Segment::TitleSlide { text }
            }
            Opener::Warning => Segment::Warning(md),
            Opener::Extra { title } => Segment::Extra { title, md },
            Opener::App => Segment::App(md),
        })
    }

    fn unclosed_error(&self) -> Error {
        match self {
            Opener::Solution => Error::UnclosedSolution,
            Opener::Slide { .. } => Error::UnclosedSlide,
            Opener::InlineSlide { .. } => Error::UnclosedInlineSlide,
            Opener::TitleSlide { .. } => Error::UnclosedTitleSlide,
            Opener::Warning => Error::UnclosedWarning,
            Opener::Extra { .. } => Error::UnclosedExtra,
            Opener::App => Error::UnclosedApp,
        }
    }
}

/// Splits a Markdown body into top-level [`Segment`]s. Directive blocks open
/// with their fence (`::: solucion`, `:::slide`, …) and close with a bare
/// `:::`. Blocks nest: an opener seen inside a block increments the depth and
/// its line is captured verbatim for the recursive renderer; the matching
/// `:::` closes the innermost block (depth returns to zero). Fences must sit
/// alone on their line. An unclosed block at end of input is an error.
pub fn split_solutions(body: &str) -> Result<Vec<Segment>> {
    let mut segments = Vec::new();
    let mut buf = String::new();
    let mut open: Option<Opener> = None;
    let mut depth: usize = 0;

    for line in body.lines() {
        let fence = line.trim_end();
        if open.is_none() {
            if let Some(op) = opener(fence) {
                if !buf.is_empty() {
                    segments.push(Segment::Markdown(std::mem::take(&mut buf)));
                }
                open = Some(op);
                depth = 1;
            } else {
                buf.push_str(line);
                buf.push('\n');
            }
            continue;
        }
        if opener(fence).is_some() {
            depth += 1;
            buf.push_str(line);
            buf.push('\n');
        } else if fence == ":::" {
            depth -= 1;
            if depth == 0 {
                let md = std::mem::take(&mut buf);
                match open.take() {
                    Some(op) => segments.push(op.into_segment(md)?),
                    None => unreachable!("open is Some in this branch"),
                }
            } else {
                buf.push_str(line);
                buf.push('\n');
            }
        } else {
            buf.push_str(line);
            buf.push('\n');
        }
    }

    if let Some(op) = open {
        return Err(op.unclosed_error());
    }
    if !buf.is_empty() {
        segments.push(Segment::Markdown(buf));
    }
    Ok(segments)
}

/// A section body rendered to HTML, plus metadata about its special blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedBody {
    pub html: String,
    pub slide_html: Vec<SlideFragment>,
    pub uses_solutions: bool,
    pub uses_slides: bool,
    pub uses_mermaid: bool,
    pub uses_apps: bool,
}

/// Renders a section's Markdown body.
///
/// `::: solucion`, `::: warning`, and `::: extra` produce their guide markup
/// and nest freely (a warning inside a solution, an extra inside a warning,
/// etc.). `:::slide` blocks are collected separately and omitted from the
/// guide HTML. `:::inline-slide` renders in BOTH places. `:::title-slide`
/// produces a slide showing only `title` (the section heading); its body must
/// be empty. Slide-family directives are top-level only — nesting one inside
/// another block is an error.
///
/// A line `{#name}` in top-level Markdown anchors the block that follows; a
/// line `{{name}}` inside a slide inserts the anchored Markdown (or an
/// exercise hero card). Anchors are scoped to the section body and resolved at
/// the top level only.
pub fn render_section_body(title: &str, body: &str) -> Result<RenderedBody> {
    let (segments, anchors) = extract_anchors(split_solutions(body)?)?;
    let mut html = String::new();
    let mut slide_html = Vec::new();
    let mut uses_solutions = false;
    let mut uses_slides = false;
    for segment in segments {
        match segment {
            Segment::Markdown(md) => html.push_str(&render_markdown(&md)),
            Segment::Solution(md) => {
                uses_solutions = true;
                let inner = render_guide_segments(&md, &mut uses_solutions)?;
                html.push_str(&render_solution(&inner));
            }
            Segment::Slide { md, light } => {
                uses_slides = true;
                slide_html.push(SlideFragment {
                    html: render_slide_content(&md, &anchors)?,
                    light,
                });
            }
            Segment::InlineSlide { md, light } => {
                uses_slides = true;
                slide_html.push(SlideFragment {
                    html: render_slide_content(&md, &anchors)?,
                    light,
                });
                html.push_str(&render_guide_segments(&md, &mut uses_solutions)?);
            }
            Segment::TitleSlide { text } => {
                uses_slides = true;
                let label = if text.is_empty() { title } else { &text };
                slide_html.push(SlideFragment {
                    html: render_title_slide(label),
                    light: false,
                });
            }
            Segment::Warning(md) => {
                let inner = render_guide_segments(&md, &mut uses_solutions)?;
                html.push_str(&render_warning(&inner));
            }
            Segment::Extra { title, md } => {
                let inner = render_guide_segments(&md, &mut uses_solutions)?;
                html.push_str(&render_extra(&title, &inner));
            }
            Segment::App(md) => {
                html.push_str(&render_app(&md));
            }
        }
    }
    // Mermaid markup is emitted by the Markdown renderer as `<pre class="mermaid">`;
    // a page only loads mermaid.js when at least one diagram is present, in the
    // guide body or in any slide.
    let uses_mermaid = html.contains("<pre class=\"mermaid\">")
        || slide_html
            .iter()
            .any(|slide| slide.html.contains("<pre class=\"mermaid\">"));
    // App widgets are emitted as `<div class="cb-app">`; a page only loads
    // apps.js when at least one app widget is present, in the guide body or
    // in any slide.
    let uses_apps = html.contains("<div class=\"cb-app\">")
        || slide_html
            .iter()
            .any(|slide| slide.html.contains("<div class=\"cb-app\">"));
    Ok(RenderedBody {
        html,
        slide_html,
        uses_solutions,
        uses_slides,
        uses_mermaid,
        uses_apps,
    })
}

/// Renders raw Markdown that may contain nested directives, in GUIDE context:
/// `::: solucion`, `::: warning`, and `::: extra` produce their guide markup;
/// nesting recurses. Slide directives are illegal here (slides live only at
/// the top level) and produce the matching nested-slide error. `{{name}}`
/// references are not expanded outside slides.
fn render_guide_segments(md: &str, uses_solutions: &mut bool) -> Result<String> {
    let mut html = String::new();
    for segment in split_solutions(md)? {
        match segment {
            Segment::Markdown(m) => html.push_str(&render_markdown(&m)),
            Segment::Solution(inner) => {
                *uses_solutions = true;
                let body = render_guide_segments(&inner, uses_solutions)?;
                html.push_str(&render_solution(&body));
            }
            Segment::Warning(inner) => {
                let body = render_guide_segments(&inner, uses_solutions)?;
                html.push_str(&render_warning(&body));
            }
            Segment::Extra { title, md } => {
                let body = render_guide_segments(&md, uses_solutions)?;
                html.push_str(&render_extra(&title, &body));
            }
            Segment::App(md) => {
                html.push_str(&render_app(&md));
            }
            Segment::Slide { .. } => return Err(Error::NestedSlide),
            Segment::InlineSlide { .. } => return Err(Error::NestedInlineSlide),
            Segment::TitleSlide { .. } => return Err(Error::NestedTitleSlide),
        }
    }
    Ok(html)
}

/// Renders a title-only slide: the section heading as a hero, centered by the
/// reveal.js layout. Styling comes from `.reveal .cb-title-slide`.
fn render_title_slide(title: &str) -> String {
    format!("<h1 class=\"cb-title-slide\">{}</h1>\n", escape_html(title))
}

/// Renders an always-visible warning admonition. The symbol comes from CSS
/// (`.cb-warning::before`), keeping markup minimal.
pub(crate) fn render_warning(inner_html: &str) -> String {
    format!("<div class=\"cb-warning\">\n{inner_html}</div>\n")
}

/// Renders a collapsible extra block as a native `<details>` element, closed
/// by default. An empty title falls back to "Contenido adicional".
pub(crate) fn render_extra(title: &str, inner_html: &str) -> String {
    let title = if title.is_empty() {
        "Contenido adicional"
    } else {
        title
    };
    format!(
        "<details class=\"cb-extra\">\n\
         <summary>{}</summary>\n\
         <div class=\"cb-extra-cuerpo\">\n{inner_html}</div>\n</details>\n",
        escape_html(title)
    )
}

/// Renders an app container: a `<div class="cb-app">` that passes the inner
/// content (custom-element tags, raw HTML) through verbatim without re-escaping.
pub(crate) fn render_app(inner: &str) -> String {
    format!("<div class=\"cb-app\">\n{inner}</div>\n")
}

pub(crate) fn render_solution(inner_html: &str) -> String {
    format!(
        "<div class=\"solucion\">\n\
         <button type=\"button\" class=\"solucion-toggle\" aria-expanded=\"false\">Ver solución</button>\n\
         <div class=\"solucion-cuerpo\" hidden>\n{inner_html}</div>\n</div>\n"
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn no_fences_yields_single_markdown_segment() {
        let body = "# Title\n\nSome text.\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Markdown("# Title\n\nSome text.\n".to_owned())]
        );
    }

    #[test]
    fn one_solution_between_prose_yields_three_segments() {
        let body = "Before.\n::: solucion\nAnswer.\n:::\nAfter.\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![
                Segment::Markdown("Before.\n".to_owned()),
                Segment::Solution("Answer.\n".to_owned()),
                Segment::Markdown("After.\n".to_owned()),
            ]
        );
    }

    #[test]
    fn solution_content_preserved_verbatim_with_trailing_newline() {
        let body = "::: solucion\nline 1\nline 2\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(segs, vec![Segment::Solution("line 1\nline 2\n".to_owned())]);
    }

    #[test]
    fn two_solutions_yield_correct_segments() {
        let body = "A\n::: solucion\nS1\n:::\nB\n::: solucion\nS2\n:::\nC\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![
                Segment::Markdown("A\n".to_owned()),
                Segment::Solution("S1\n".to_owned()),
                Segment::Markdown("B\n".to_owned()),
                Segment::Solution("S2\n".to_owned()),
                Segment::Markdown("C\n".to_owned()),
            ]
        );
    }

    #[test]
    fn empty_body_yields_empty_vec() {
        assert_eq!(split_solutions("").unwrap(), vec![]);
    }

    #[test]
    fn unclosed_block_errors() {
        let body = "Before\n::: solucion\nAnswer\n";
        assert!(matches!(
            split_solutions(body),
            Err(Error::UnclosedSolution)
        ));
    }

    #[test]
    fn trailing_whitespace_on_fence_still_counts() {
        let body = "::: solucion   \nAnswer\n:::   \n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(segs, vec![Segment::Solution("Answer\n".to_owned())]);
    }

    #[test]
    fn bare_close_fence_outside_block_is_plain_markdown() {
        let body = "Text\n:::\nMore text\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Markdown("Text\n:::\nMore text\n".to_owned())]
        );
    }

    // ── nesting: inner content captured verbatim ──────────────────────────

    #[test]
    fn nested_block_payload_captured_verbatim() {
        let body = "::: warning\nCuidado.\n::: extra Detalle\nTexto.\n:::\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Warning(
                "Cuidado.\n::: extra Detalle\nTexto.\n:::\n".to_owned()
            )]
        );
    }

    #[test]
    fn innermost_close_matches_innermost_open() {
        // Two `:::` close warning then solution; trailing markdown follows.
        let body = "::: solucion\n::: warning\nW.\n:::\nrest of answer\n:::\nAfter.\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![
                Segment::Solution("::: warning\nW.\n:::\nrest of answer\n".to_owned()),
                Segment::Markdown("After.\n".to_owned()),
            ]
        );
    }

    #[test]
    fn slide_with_nested_warning_payload_is_verbatim() {
        let body = ":::slide\n## T\n::: warning\nW.\n:::\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Slide {
                md: "## T\n::: warning\nW.\n:::\n".to_owned(),
                light: false,
            }]
        );
    }

    // ── warning / extra fences ────────────────────────────────────────────

    #[test]
    fn warning_with_arguments_is_plain_markdown() {
        let body = "::: warning extra-text\nX.\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Markdown(
                "::: warning extra-text\nX.\n:::\n".to_owned()
            )]
        );
    }

    #[test]
    fn unclosed_warning_errors() {
        assert!(matches!(
            split_solutions("::: warning\nX.\n"),
            Err(Error::UnclosedWarning)
        ));
    }

    #[test]
    fn extra_with_title_yields_extra_segment() {
        let body = "::: extra ¿Qué es un repositorio remoto?\nTexto.\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Extra {
                title: "¿Qué es un repositorio remoto?".to_owned(),
                md: "Texto.\n".to_owned(),
            }]
        );
    }

    #[test]
    fn extra_without_title_yields_empty_title() {
        let body = "::: extra\nTexto.\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Extra {
                title: String::new(),
                md: "Texto.\n".to_owned(),
            }]
        );
    }

    #[test]
    fn extra_title_trims_whitespace() {
        let body = "::: extra   Comandos de git   \nX.\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Extra {
                title: "Comandos de git".to_owned(),
                md: "X.\n".to_owned(),
            }]
        );
    }

    #[test]
    fn unclosed_extra_errors() {
        assert!(matches!(
            split_solutions("::: extra T\nX.\n"),
            Err(Error::UnclosedExtra)
        ));
    }

    // ── app fence ─────────────────────────────────────────────────────────

    #[test]
    fn app_block_produces_app_segment() {
        let body = ":::app\n<cb-counter key=\"x\"></cb-counter>\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::App(
                "<cb-counter key=\"x\"></cb-counter>\n".to_owned()
            )]
        );
    }

    #[test]
    fn unclosed_app_errors() {
        assert!(matches!(
            split_solutions(":::app\n<cb-counter></cb-counter>\n"),
            Err(Error::UnclosedApp)
        ));
    }

    #[test]
    fn app_renders_cb_app_div_with_verbatim_inner() {
        let body = ":::app\n<cb-cpu-burst></cb-cpu-burst>\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.html.contains("<div class=\"cb-app\">"));
        assert!(result.html.contains("<cb-cpu-burst></cb-cpu-burst>"));
        assert!(result.uses_apps);
    }

    #[test]
    fn body_without_app_uses_apps_is_false() {
        let result = render_section_body("Sección", "Just plain text.\n").unwrap();
        assert!(!result.uses_apps);
    }

    #[test]
    fn body_with_cb_app_div_sets_uses_apps_true() {
        let body = ":::app\n<cb-counter key=\"x\"></cb-counter>\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.uses_apps);
    }

    // ── slide / inline-slide / title-slide fences ─────────────────────────

    #[test]
    fn slide_block_produces_slide_segment() {
        let body = "Intro.\n:::slide\n## Slide 1\nContent.\n:::\nOutro.\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![
                Segment::Markdown("Intro.\n".to_owned()),
                Segment::Slide {
                    md: "## Slide 1\nContent.\n".to_owned(),
                    light: false
                },
                Segment::Markdown("Outro.\n".to_owned()),
            ]
        );
    }

    #[test]
    fn unclosed_slide_block_errors() {
        let body = ":::slide\nNo closing fence.\n";
        assert!(matches!(split_solutions(body), Err(Error::UnclosedSlide)));
    }

    #[test]
    fn slide_light_modifier_produces_light_fragment() {
        let body = ":::slide light\n## Light slide\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Slide {
                md: "## Light slide\n".to_owned(),
                light: true
            }]
        );
    }

    #[test]
    fn inline_slide_produces_inline_slide_segment() {
        let body = ":::inline-slide\n## Resumen\nTexto.\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::InlineSlide {
                md: "## Resumen\nTexto.\n".to_owned(),
                light: false
            }]
        );
    }

    #[test]
    fn inline_slide_light_modifier_produces_light_segment() {
        let body = ":::inline-slide light\n## Resumen\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::InlineSlide {
                md: "## Resumen\n".to_owned(),
                light: true
            }]
        );
    }

    #[test]
    fn unclosed_inline_slide_errors() {
        assert!(matches!(
            split_solutions(":::inline-slide\nX.\n"),
            Err(Error::UnclosedInlineSlide)
        ));
    }

    #[test]
    fn title_slide_produces_title_slide_segment() {
        let body = ":::title-slide\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::TitleSlide {
                text: String::new()
            }]
        );
    }

    #[test]
    fn title_slide_with_only_blank_lines_is_empty() {
        let body = ":::title-slide\n\n  \n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::TitleSlide {
                text: String::new()
            }]
        );
    }

    #[test]
    fn title_slide_with_custom_text_captures_it() {
        let body = ":::title-slide Semana 1\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::TitleSlide {
                text: "Semana 1".to_owned()
            }]
        );
    }

    #[test]
    fn title_slide_with_text_still_rejects_body() {
        assert!(matches!(
            split_solutions(":::title-slide Semana 1\nTexto.\n:::\n"),
            Err(Error::TitleSlideNotEmpty)
        ));
    }

    #[test]
    fn title_slide_with_content_errors() {
        assert!(matches!(
            split_solutions(":::title-slide\nTexto.\n:::\n"),
            Err(Error::TitleSlideNotEmpty)
        ));
    }

    #[test]
    fn unclosed_title_slide_errors() {
        assert!(matches!(
            split_solutions(":::title-slide\n"),
            Err(Error::UnclosedTitleSlide)
        ));
    }

    // ── render_section_body tests ─────────────────────────────────────────

    #[test]
    fn body_without_solutions_sets_flag_false_and_has_no_solucion_class() {
        let result = render_section_body("Sección", "Just **plain** text.\n").unwrap();
        assert!(!result.uses_solutions);
        assert!(!result.html.contains("class=\"solucion\""));
    }

    #[test]
    fn body_with_solution_sets_flag_true_and_emits_button() {
        let body = "Intro\n::: solucion\n**negrita**\n:::\nOutro\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.uses_solutions);
        assert!(result.html.contains("Ver solución"));
        assert!(result.html.contains("aria-expanded=\"false\""));
        assert!(result.html.contains("hidden"));
    }

    #[test]
    fn solution_inner_markdown_is_rendered() {
        let body = "::: solucion\n**negrita**\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.html.contains("<strong>negrita</strong>"));
    }

    #[test]
    fn split_errors_propagate_from_render_section_body() {
        let body = "::: solucion\nno closing fence\n";
        assert!(matches!(
            render_section_body("Sección", body),
            Err(Error::UnclosedSolution)
        ));
    }

    #[test]
    fn render_section_body_slide_omitted_from_guide_html() {
        let body = "Before.\n:::slide\n**Bold**\n:::\nAfter.\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(!result.html.contains("<strong>Bold</strong>"));
        assert!(result.html.contains("Before."));
        assert!(result.html.contains("After."));
        assert!(result.uses_slides);
        assert_eq!(result.slide_html.len(), 1);
        assert!(result.slide_html[0].html.contains("<strong>Bold</strong>"));
    }

    #[test]
    fn render_section_body_light_slide_sets_light_true() {
        let body = ":::slide light\n**Bold**\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert_eq!(result.slide_html.len(), 1);
        assert!(result.slide_html[0].light);
        assert!(result.slide_html[0].html.contains("<strong>Bold</strong>"));
    }

    #[test]
    fn inline_slide_renders_in_both_guide_and_slides() {
        let body = "Before.\n:::inline-slide\n**Bold**\n:::\nAfter.\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.uses_slides);
        assert_eq!(result.slide_html.len(), 1);
        assert!(result.slide_html[0].html.contains("<strong>Bold</strong>"));
        assert!(result.html.contains("<strong>Bold</strong>"));
        assert!(result.html.contains("Before."));
        assert!(result.html.contains("After."));
    }

    #[test]
    fn title_slide_renders_section_title_only() {
        let body = "Intro.\n:::title-slide\n:::\n";
        let result = render_section_body("El origen del código", body).unwrap();
        assert!(result.uses_slides);
        assert_eq!(result.slide_html.len(), 1);
        assert!(
            result.slide_html[0]
                .html
                .contains("class=\"cb-title-slide\">El origen del código</h1>")
        );
        assert!(!result.slide_html[0].light);
    }

    #[test]
    fn title_slide_custom_text_overrides_section_title() {
        let body = ":::title-slide Semana 1\n:::\n";
        let result = render_section_body("Introducción", body).unwrap();
        assert!(
            result.slide_html[0]
                .html
                .contains("class=\"cb-title-slide\">Semana 1</h1>")
        );
    }

    #[test]
    fn title_slide_escapes_section_title() {
        let body = ":::title-slide\n:::\n";
        let result = render_section_body("Semana <1> & más", body).unwrap();
        assert!(
            result.slide_html[0]
                .html
                .contains("Semana &lt;1&gt; &amp; más")
        );
    }

    #[test]
    fn render_section_body_no_slides_has_empty_slide_html() {
        let result = render_section_body("Sección", "Plain text.\n").unwrap();
        assert!(!result.uses_slides);
        assert!(result.slide_html.is_empty());
    }

    // ── nested rendering (guide context) ──────────────────────────────────

    #[test]
    fn warning_inside_solution_renders_both() {
        let body = "::: solucion\nRespuesta.\n::: warning\nCuidado.\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.uses_solutions);
        assert!(result.html.contains("class=\"solucion\""));
        assert!(result.html.contains("<div class=\"cb-warning\">"));
        assert!(result.html.contains("Cuidado."));
    }

    #[test]
    fn extra_inside_warning_renders_nested_details() {
        let body = "::: warning\nAviso.\n::: extra Más\nDetalle.\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.html.contains("<div class=\"cb-warning\">"));
        assert!(result.html.contains("<details class=\"cb-extra\">"));
        assert!(result.html.contains("<summary>Más</summary>"));
        assert!(result.html.contains("Detalle."));
    }

    #[test]
    fn deeply_nested_solution_inside_extra_inside_warning() {
        let body = "::: warning\nA.\n::: extra T\nB.\n::: solucion\nC.\n:::\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.uses_solutions);
        assert!(result.html.contains("Ver solución"));
        assert!(result.html.contains("C."));
    }

    #[test]
    fn slide_nested_in_warning_errors() {
        let body = "::: warning\n:::slide\n:::\n:::\n";
        assert!(matches!(
            render_section_body("Sección", body),
            Err(Error::NestedSlide)
        ));
    }

    #[test]
    fn slide_nested_in_solution_errors() {
        let body = "::: solucion\n:::slide\n:::\n:::\n";
        assert!(matches!(
            render_section_body("Sección", body),
            Err(Error::NestedSlide)
        ));
    }

    // ── warning / extra rendering ─────────────────────────────────────────

    #[test]
    fn warning_renders_admonition_div_with_inner_markdown() {
        let body = "::: warning\n**Atención**: [aviso](https://example.com).\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.html.contains("<div class=\"cb-warning\">"));
        assert!(result.html.contains("<strong>Atención</strong>"));
        assert!(result.html.contains("target=\"_blank\""));
    }

    #[test]
    fn extra_renders_closed_details_with_summary_title() {
        let body = "::: extra ¿Qué es un remoto?\nUn remoto es…\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.html.contains("<details class=\"cb-extra\">"));
        assert!(
            result
                .html
                .contains("<summary>¿Qué es un remoto?</summary>")
        );
        assert!(result.html.contains("Un remoto es…"));
        assert!(!result.html.contains("<details open"));
    }

    #[test]
    fn extra_without_title_uses_fallback() {
        let body = "::: extra\nX.\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(
            result
                .html
                .contains("<summary>Contenido adicional</summary>")
        );
    }

    #[test]
    fn extra_title_is_html_escaped() {
        let body = "::: extra Uso de <code> & otros\nX.\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(
            result
                .html
                .contains("<summary>Uso de &lt;code&gt; &amp; otros</summary>")
        );
    }

    #[test]
    fn warning_and_extra_omitted_from_slides_and_do_not_set_flags() {
        let body = "::: warning\nW.\n:::\n::: extra T\nE.\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(!result.uses_solutions);
        assert!(!result.uses_slides);
        assert!(result.slide_html.is_empty());
    }

    #[test]
    fn app_block_inside_slide_sets_uses_apps_via_slide_html_detection() {
        // The :::app block here lives inside a :::slide fence, so it is
        // rendered into slide_html rather than the guide html.  The
        // `slide_html.iter().any(...)` branch in render_section_body must
        // detect the `<div class="cb-app">` in the slide fragment and set
        // uses_apps = true even though the guide html contains no app div.
        let body =
            ":::slide\n## Herramienta\n:::app\n<cb-counter key=\"x\"></cb-counter>\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(
            !result.html.contains("<div class=\"cb-app\">"),
            "app div must not appear in guide html when inside a slide fence"
        );
        assert!(
            result
                .slide_html
                .iter()
                .any(|s| s.html.contains("<div class=\"cb-app\">")),
            "slide fragment must contain the cb-app div"
        );
        assert!(
            result.uses_apps,
            "uses_apps must be true when cb-app appears only in slide_html"
        );
    }
}
