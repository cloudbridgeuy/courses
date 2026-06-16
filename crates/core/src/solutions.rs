use crate::error::{Error, Result};
use crate::markdown::render_markdown;
use crate::render::escape_html;
use crate::slides::{extract_anchors, render_slide};

/// One segment of a section body: plain Markdown, a hidden solution, a
/// slides-only block, an inline slide (rendered in both guide and slides), a
/// title-only slide, an always-visible warning, or a collapsible extra block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Markdown(String),
    Solution(String),
    Slide { md: String, light: bool },
    InlineSlide { md: String, light: bool },
    TitleSlide,
    Warning(String),
    Extra { title: String, md: String },
}

/// A rendered slide HTML fragment with its display variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideFragment {
    pub html: String,
    pub light: bool,
}

/// Splits a Markdown body on `::: solucion`, `:::slide`, `:::inline-slide`,
/// `:::title-slide`, `::: warning`, and `::: extra <title>` fenced blocks
/// (closed by `:::`).
///
/// Fences must sit alone on their line. Nesting is rejected; an unclosed
/// block is rejected.
///
/// Note: the slide fences (`:::slide`, `:::inline-slide`, `:::title-slide`)
/// take no space; the others use a space. `:::slide` and `:::inline-slide`
/// accept an optional `light` modifier. `::: warning` takes no arguments —
/// with trailing text it is plain Markdown. `::: extra` takes an optional
/// title after the keyword.
pub fn split_solutions(body: &str) -> Result<Vec<Segment>> {
    enum State {
        Outside,
        InSolution,
        InSlide { light: bool },
        InInlineSlide { light: bool },
        InTitleSlide,
        InWarning,
        InExtra { title: String },
    }

    fn opener(fence: &str) -> Option<State> {
        if fence == "::: solucion" {
            Some(State::InSolution)
        } else if fence == ":::slide" || fence == ":::slide light" {
            Some(State::InSlide {
                light: fence == ":::slide light",
            })
        } else if fence == ":::inline-slide" || fence == ":::inline-slide light" {
            Some(State::InInlineSlide {
                light: fence == ":::inline-slide light",
            })
        } else if fence == ":::title-slide" {
            Some(State::InTitleSlide)
        } else if fence == "::: warning" {
            Some(State::InWarning)
        } else if let Some(rest) = fence.strip_prefix("::: extra") {
            (rest.is_empty() || rest.starts_with(' ')).then(|| State::InExtra {
                title: rest.trim().to_owned(),
            })
        } else {
            None
        }
    }

    fn nested_error(state: &State) -> Error {
        match state {
            State::InSolution => Error::NestedSolution,
            State::InSlide { .. } => Error::NestedSlide,
            State::InInlineSlide { .. } => Error::NestedInlineSlide,
            State::InTitleSlide => Error::NestedTitleSlide,
            State::InWarning => Error::NestedWarning,
            State::InExtra { .. } => Error::NestedExtra,
            State::Outside => unreachable!("opener never returns Outside"),
        }
    }

    let mut segments = Vec::new();
    let mut state = State::Outside;
    let mut buf = String::new();

    for line in body.lines() {
        let fence = line.trim_end();
        match state {
            State::Outside => {
                if let Some(open) = opener(fence) {
                    if !buf.is_empty() {
                        segments.push(Segment::Markdown(std::mem::take(&mut buf)));
                    }
                    state = open;
                } else {
                    buf.push_str(line);
                    buf.push('\n');
                }
            }
            ref open_state => {
                if let Some(inner) = opener(fence) {
                    return Err(nested_error(&inner));
                } else if fence == ":::" {
                    let md = std::mem::take(&mut buf);
                    let segment = match std::mem::replace(&mut state, State::Outside) {
                        State::InSolution => Segment::Solution(md),
                        State::InSlide { light } => Segment::Slide { md, light },
                        State::InInlineSlide { light } => Segment::InlineSlide { md, light },
                        State::InTitleSlide => {
                            if !md.trim().is_empty() {
                                return Err(Error::TitleSlideNotEmpty);
                            }
                            Segment::TitleSlide
                        }
                        State::InWarning => Segment::Warning(md),
                        State::InExtra { title } => Segment::Extra { title, md },
                        State::Outside => unreachable!("matched non-Outside arm"),
                    };
                    segments.push(segment);
                } else {
                    let _ = open_state;
                    buf.push_str(line);
                    buf.push('\n');
                }
            }
        }
    }

    match state {
        State::InSolution => return Err(Error::UnclosedSolution),
        State::InSlide { .. } => return Err(Error::UnclosedSlide),
        State::InInlineSlide { .. } => return Err(Error::UnclosedInlineSlide),
        State::InTitleSlide => return Err(Error::UnclosedTitleSlide),
        State::InWarning => return Err(Error::UnclosedWarning),
        State::InExtra { .. } => return Err(Error::UnclosedExtra),
        State::Outside => {}
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
}

/// Renders a section's Markdown body, expanding `::: solucion` blocks into
/// toggle markup (button plus hidden panel).
///
/// `:::slide` blocks are collected separately and omitted from the guide HTML.
/// `:::inline-slide` blocks render in BOTH places: as a slide fragment and,
/// inline, as guide document content. `:::title-slide` produces a slide that
/// shows only `title` (the section heading); its body must be empty.
/// `::: warning` renders as an always-visible admonition div; `::: extra
/// <title>` renders as a closed `<details>` block — both are guide-only.
///
/// A line `{#name}` in plain Markdown anchors the block that follows (until a
/// blank line; a whole code fence; or, when the block opens with a heading,
/// the whole subsection up to the next heading, `---` rule, or `::: solucion`
/// block). The marker line is stripped from the guide HTML; the block itself
/// still renders in the guide. Inside a `:::slide` block, a line `{{name}}`
/// inserts the anchored Markdown. Anchors are scoped to the section body.
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
                html.push_str(&render_solution(&render_markdown(&md)));
            }
            Segment::Slide { md, light } => {
                uses_slides = true;
                slide_html.push(SlideFragment {
                    html: render_slide(&md, &anchors)?,
                    light,
                });
            }
            Segment::InlineSlide { md, light } => {
                uses_slides = true;
                slide_html.push(SlideFragment {
                    html: render_slide(&md, &anchors)?,
                    light,
                });
                html.push_str(&render_markdown(&md));
            }
            Segment::TitleSlide => {
                uses_slides = true;
                slide_html.push(SlideFragment {
                    html: render_title_slide(title),
                    light: false,
                });
            }
            Segment::Warning(md) => html.push_str(&render_warning(&render_markdown(&md))),
            Segment::Extra { title, md } => {
                html.push_str(&render_extra(&title, &render_markdown(&md)));
            }
        }
    }
    Ok(RenderedBody {
        html,
        slide_html,
        uses_solutions,
        uses_slides,
    })
}

/// Renders a title-only slide: the section heading as a hero, centered by the
/// reveal.js layout. Styling comes from `.reveal .cb-title-slide`.
fn render_title_slide(title: &str) -> String {
    format!("<h1 class=\"cb-title-slide\">{}</h1>\n", escape_html(title))
}

/// Renders an always-visible warning admonition. The symbol comes from CSS
/// (`.cb-warning::before`), keeping markup minimal.
fn render_warning(inner_html: &str) -> String {
    format!("<div class=\"cb-warning\">\n{inner_html}</div>\n")
}

/// Renders a collapsible extra block as a native `<details>` element, closed
/// by default. An empty title falls back to "Contenido adicional".
fn render_extra(title: &str, inner_html: &str) -> String {
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
    fn solution_at_start_of_body() {
        let body = "::: solucion\nAnswer\n:::\nAfter\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![
                Segment::Solution("Answer\n".to_owned()),
                Segment::Markdown("After\n".to_owned()),
            ]
        );
    }

    #[test]
    fn solution_at_end_of_body() {
        let body = "Before\n::: solucion\nAnswer\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![
                Segment::Markdown("Before\n".to_owned()),
                Segment::Solution("Answer\n".to_owned()),
            ]
        );
    }

    #[test]
    fn empty_body_yields_empty_vec() {
        assert_eq!(split_solutions("").unwrap(), vec![]);
    }

    #[test]
    fn nested_solution_block_errors() {
        let body = "::: solucion\n::: solucion\n:::\n:::\n";
        assert!(matches!(split_solutions(body), Err(Error::NestedSolution)));
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
    fn bare_close_fence_outside_solution_is_plain_markdown() {
        let body = "Text\n:::\nMore text\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Markdown("Text\n:::\nMore text\n".to_owned())]
        );
    }

    // ── warning / extra fences ────────────────────────────────────────────

    #[test]
    fn warning_between_prose_yields_three_segments() {
        let body = "Before.\n::: warning\nDanger.\n:::\nAfter.\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![
                Segment::Markdown("Before.\n".to_owned()),
                Segment::Warning("Danger.\n".to_owned()),
                Segment::Markdown("After.\n".to_owned()),
            ]
        );
    }

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
    fn nested_warning_inside_warning_errors() {
        assert!(matches!(
            split_solutions("::: warning\n::: warning\n:::\n:::\n"),
            Err(Error::NestedWarning)
        ));
    }

    #[test]
    fn warning_inside_solution_errors() {
        assert!(matches!(
            split_solutions("::: solucion\n::: warning\n:::\n:::\n"),
            Err(Error::NestedWarning)
        ));
    }

    #[test]
    fn solution_inside_warning_errors() {
        assert!(matches!(
            split_solutions("::: warning\n::: solucion\n:::\n:::\n"),
            Err(Error::NestedSolution)
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

    #[test]
    fn extra_inside_slide_errors() {
        assert!(matches!(
            split_solutions(":::slide\n::: extra T\n:::\n:::\n"),
            Err(Error::NestedExtra)
        ));
    }

    #[test]
    fn warning_inside_slide_errors() {
        assert!(matches!(
            split_solutions(":::slide\n::: warning\n:::\n:::\n"),
            Err(Error::NestedWarning)
        ));
    }

    #[test]
    fn extra_inside_warning_errors() {
        assert!(matches!(
            split_solutions("::: warning\n::: extra T\n:::\n:::\n"),
            Err(Error::NestedExtra)
        ));
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
    fn slide_and_solution_in_same_body() {
        let body = ":::slide\nSlide content.\n:::\n::: solucion\nAnswer.\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![
                Segment::Slide {
                    md: "Slide content.\n".to_owned(),
                    light: false
                },
                Segment::Solution("Answer.\n".to_owned()),
            ]
        );
    }

    #[test]
    fn unclosed_slide_block_errors() {
        let body = ":::slide\nNo closing fence.\n";
        assert!(matches!(split_solutions(body), Err(Error::UnclosedSlide)));
    }

    #[test]
    fn nested_slide_inside_slide_errors() {
        let body = ":::slide\n:::slide\n:::\n:::\n";
        assert!(matches!(split_solutions(body), Err(Error::NestedSlide)));
    }

    #[test]
    fn slide_inside_solution_errors() {
        let body = "::: solucion\n:::slide\n:::\n:::\n";
        assert!(matches!(split_solutions(body), Err(Error::NestedSlide)));
    }

    #[test]
    fn solution_inside_slide_errors() {
        let body = ":::slide\n::: solucion\n:::\n:::\n";
        assert!(matches!(split_solutions(body), Err(Error::NestedSolution)));
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
    fn slide_without_modifier_produces_dark_fragment() {
        let body = ":::slide\n## Dark slide\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Slide {
                md: "## Dark slide\n".to_owned(),
                light: false
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
    fn nested_inline_slide_errors() {
        assert!(matches!(
            split_solutions(":::inline-slide\n:::inline-slide\n:::\n:::\n"),
            Err(Error::NestedInlineSlide)
        ));
    }

    #[test]
    fn title_slide_produces_title_slide_segment() {
        let body = ":::title-slide\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(segs, vec![Segment::TitleSlide]);
    }

    #[test]
    fn title_slide_with_only_blank_lines_is_empty() {
        let body = ":::title-slide\n\n  \n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(segs, vec![Segment::TitleSlide]);
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

    #[test]
    fn nested_title_slide_errors() {
        assert!(matches!(
            split_solutions(":::title-slide\n:::title-slide\n:::\n:::\n"),
            Err(Error::NestedTitleSlide)
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
        // also present inline in the guide document
        assert!(result.html.contains("<strong>Bold</strong>"));
        assert!(result.html.contains("Before."));
        assert!(result.html.contains("After."));
    }

    #[test]
    fn inline_slide_light_sets_light_fragment() {
        let body = ":::inline-slide light\n## Resumen\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert_eq!(result.slide_html.len(), 1);
        assert!(result.slide_html[0].light);
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
}
