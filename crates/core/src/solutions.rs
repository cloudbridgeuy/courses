use crate::error::{Error, Result};
use crate::markdown::render_markdown;

/// One segment of a section body: plain Markdown, a hidden solution, or a slide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Markdown(String),
    Solution(String),
    Slide(String),
}

/// Splits a Markdown body on `::: solucion` and `:::slide` / `:::` fenced blocks.
///
/// Fences must sit alone on their line. Nesting is rejected; an unclosed
/// block is rejected.
///
/// Note: `":::slide"` has no space; `"::: solucion"` uses a space.
/// Writing `"::: slide"` (with a space) is silently treated as plain Markdown.
pub fn split_solutions(body: &str) -> Result<Vec<Segment>> {
    enum State {
        Outside,
        InSolution,
        InSlide,
    }

    let mut segments = Vec::new();
    let mut state = State::Outside;
    let mut buf = String::new();

    for line in body.lines() {
        let fence = line.trim_end();
        match state {
            State::Outside => {
                if fence == "::: solucion" {
                    if !buf.is_empty() {
                        segments.push(Segment::Markdown(std::mem::take(&mut buf)));
                    }
                    state = State::InSolution;
                } else if fence == ":::slide" {
                    if !buf.is_empty() {
                        segments.push(Segment::Markdown(std::mem::take(&mut buf)));
                    }
                    state = State::InSlide;
                } else {
                    buf.push_str(line);
                    buf.push('\n');
                }
            }
            State::InSolution => {
                if fence == "::: solucion" {
                    return Err(Error::NestedSolution);
                } else if fence == ":::slide" {
                    return Err(Error::NestedSlide);
                } else if fence == ":::" {
                    segments.push(Segment::Solution(std::mem::take(&mut buf)));
                    state = State::Outside;
                } else {
                    buf.push_str(line);
                    buf.push('\n');
                }
            }
            State::InSlide => {
                if fence == ":::slide" {
                    return Err(Error::NestedSlide);
                } else if fence == "::: solucion" {
                    return Err(Error::NestedSolution);
                } else if fence == ":::" {
                    segments.push(Segment::Slide(std::mem::take(&mut buf)));
                    state = State::Outside;
                } else {
                    buf.push_str(line);
                    buf.push('\n');
                }
            }
        }
    }

    match state {
        State::InSolution => return Err(Error::UnclosedSolution),
        State::InSlide => return Err(Error::UnclosedSlide),
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
    pub slide_html: Vec<String>,
    pub uses_solutions: bool,
    pub uses_slides: bool,
}

/// Renders a section's Markdown body, expanding `::: solucion` blocks into
/// toggle markup (button plus hidden panel). `:::slide` blocks are collected
/// separately and omitted from the guide HTML.
pub fn render_section_body(body: &str) -> Result<RenderedBody> {
    let segments = split_solutions(body)?;
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
            Segment::Slide(md) => {
                uses_slides = true;
                slide_html.push(render_markdown(&md));
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

fn render_solution(inner_html: &str) -> String {
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

    // ── render_section_body tests ─────────────────────────────────────────

    #[test]
    fn body_without_solutions_sets_flag_false_and_has_no_solucion_class() {
        let result = render_section_body("Just **plain** text.\n").unwrap();
        assert!(!result.uses_solutions);
        assert!(!result.html.contains("class=\"solucion\""));
    }

    #[test]
    fn body_with_solution_sets_flag_true_and_emits_button() {
        let body = "Intro\n::: solucion\n**negrita**\n:::\nOutro\n";
        let result = render_section_body(body).unwrap();
        assert!(result.uses_solutions);
        assert!(result.html.contains("Ver solución"));
        assert!(result.html.contains("aria-expanded=\"false\""));
        assert!(result.html.contains("hidden"));
    }

    #[test]
    fn solution_inner_markdown_is_rendered() {
        let body = "::: solucion\n**negrita**\n:::\n";
        let result = render_section_body(body).unwrap();
        assert!(result.html.contains("<strong>negrita</strong>"));
    }

    #[test]
    fn split_errors_propagate_from_render_section_body() {
        let body = "::: solucion\nno closing fence\n";
        assert!(matches!(
            render_section_body(body),
            Err(Error::UnclosedSolution)
        ));
    }

    #[test]
    fn slide_block_produces_slide_segment() {
        let body = "Intro.\n:::slide\n## Slide 1\nContent.\n:::\nOutro.\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![
                Segment::Markdown("Intro.\n".to_owned()),
                Segment::Slide("## Slide 1\nContent.\n".to_owned()),
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
                Segment::Slide("Slide content.\n".to_owned()),
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
    fn render_section_body_slide_omitted_from_guide_html() {
        let body = "Before.\n:::slide\n**Bold**\n:::\nAfter.\n";
        let result = render_section_body(body).unwrap();
        assert!(!result.html.contains("<strong>Bold</strong>"));
        assert!(result.html.contains("Before."));
        assert!(result.html.contains("After."));
        assert!(result.uses_slides);
        assert_eq!(result.slide_html.len(), 1);
        assert!(result.slide_html[0].contains("<strong>Bold</strong>"));
    }

    #[test]
    fn render_section_body_no_slides_has_empty_slide_html() {
        let result = render_section_body("Plain text.\n").unwrap();
        assert!(!result.uses_slides);
        assert!(result.slide_html.is_empty());
    }
}
