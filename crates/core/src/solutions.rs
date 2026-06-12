use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::markdown::render_markdown;

/// One segment of a section body: plain Markdown, a hidden solution, or a slide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Markdown(String),
    Solution(String),
    Slide { md: String, light: bool },
}

/// A rendered slide HTML fragment with its display variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideFragment {
    pub html: String,
    pub light: bool,
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
        InSlide { light: bool },
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
                } else if fence == ":::slide" || fence == ":::slide light" {
                    if !buf.is_empty() {
                        segments.push(Segment::Markdown(std::mem::take(&mut buf)));
                    }
                    state = State::InSlide {
                        light: fence == ":::slide light",
                    };
                } else {
                    buf.push_str(line);
                    buf.push('\n');
                }
            }
            State::InSolution => {
                if fence == "::: solucion" {
                    return Err(Error::NestedSolution);
                } else if fence == ":::slide" || fence == ":::slide light" {
                    return Err(Error::NestedSlide);
                } else if fence == ":::" {
                    segments.push(Segment::Solution(std::mem::take(&mut buf)));
                    state = State::Outside;
                } else {
                    buf.push_str(line);
                    buf.push('\n');
                }
            }
            State::InSlide { light } => {
                if fence == ":::slide" || fence == ":::slide light" {
                    return Err(Error::NestedSlide);
                } else if fence == "::: solucion" {
                    return Err(Error::NestedSolution);
                } else if fence == ":::" {
                    segments.push(Segment::Slide {
                        md: std::mem::take(&mut buf),
                        light,
                    });
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
        State::InSlide { .. } => return Err(Error::UnclosedSlide),
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
/// toggle markup (button plus hidden panel). `:::slide` blocks are collected
/// separately and omitted from the guide HTML.
///
/// A line `{#name}` in plain Markdown anchors the block that follows (until a
/// blank line, or the closing fence for a code fence). The marker line is
/// stripped from the guide HTML; the block itself still renders in the guide.
/// Inside a `:::slide` block, a line `{{name}}` inserts the anchored Markdown.
/// Anchors are scoped to the section body.
pub fn render_section_body(body: &str) -> Result<RenderedBody> {
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
                let md = substitute_refs(&md, &anchors)?;
                slide_html.push(SlideFragment {
                    html: render_markdown(&md),
                    light,
                });
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

/// Collects `{#name}` anchors from Markdown segments, stripping the marker
/// lines while leaving the anchored blocks in place.
fn extract_anchors(segments: Vec<Segment>) -> Result<(Vec<Segment>, HashMap<String, String>)> {
    let mut anchors = HashMap::new();
    let mut out = Vec::with_capacity(segments.len());
    for segment in segments {
        match segment {
            Segment::Markdown(md) => {
                out.push(Segment::Markdown(strip_anchors(&md, &mut anchors)?));
            }
            other => out.push(other),
        }
    }
    Ok((out, anchors))
}

fn strip_anchors(md: &str, anchors: &mut HashMap<String, String>) -> Result<String> {
    let lines: Vec<&str> = md.lines().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < lines.len() {
        let Some(name) = parse_anchor_marker(lines[i]) else {
            out.push_str(lines[i]);
            out.push('\n');
            i += 1;
            continue;
        };
        let start = i + 1;
        let end = anchored_block_end(&lines, start);
        if end == start {
            return Err(Error::EmptyAnchor(name.to_owned()));
        }
        let block: String = lines[start..end].iter().map(|l| format!("{l}\n")).collect();
        if anchors.insert(name.to_owned(), block.clone()).is_some() {
            return Err(Error::DuplicateAnchor(name.to_owned()));
        }
        out.push_str(&block);
        i = end;
    }
    Ok(out)
}

/// End (exclusive) of the block starting at `start`: a whole code fence, or
/// the run of lines up to the next blank line.
fn anchored_block_end(lines: &[&str], start: usize) -> usize {
    if lines
        .get(start)
        .is_some_and(|l| l.trim_start().starts_with("```"))
    {
        let mut end = start + 1;
        while end < lines.len() && !lines[end].trim_start().starts_with("```") {
            end += 1;
        }
        return (end + 1).min(lines.len());
    }
    let mut end = start;
    while end < lines.len() && !lines[end].trim().is_empty() {
        end += 1;
    }
    end
}

/// Parses a `{#name}` marker line; returns the name if the line is one.
fn parse_anchor_marker(line: &str) -> Option<&str> {
    let name = line.trim().strip_prefix("{#")?.strip_suffix('}')?;
    is_valid_anchor_name(name).then_some(name)
}

fn is_valid_anchor_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Replaces `{{name}}` lines in slide Markdown with the anchored block.
fn substitute_refs(md: &str, anchors: &HashMap<String, String>) -> Result<String> {
    let mut out = String::new();
    for line in md.lines() {
        let reference = line
            .trim()
            .strip_prefix("{{")
            .and_then(|r| r.strip_suffix("}}"))
            .map(str::trim)
            .filter(|name| is_valid_anchor_name(name));
        match reference {
            Some(name) => {
                let block = anchors
                    .get(name)
                    .ok_or_else(|| Error::UnknownSlideRef(name.to_owned()))?;
                out.push_str(block);
            }
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    Ok(out)
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
    fn render_section_body_slide_omitted_from_guide_html() {
        let body = "Before.\n:::slide\n**Bold**\n:::\nAfter.\n";
        let result = render_section_body(body).unwrap();
        assert!(!result.html.contains("<strong>Bold</strong>"));
        assert!(result.html.contains("Before."));
        assert!(result.html.contains("After."));
        assert!(result.uses_slides);
        assert_eq!(result.slide_html.len(), 1);
        assert!(result.slide_html[0].html.contains("<strong>Bold</strong>"));
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
    fn render_section_body_light_slide_sets_light_true() {
        let body = ":::slide light\n**Bold**\n:::\n";
        let result = render_section_body(body).unwrap();
        assert_eq!(result.slide_html.len(), 1);
        assert!(result.slide_html[0].light);
        assert!(result.slide_html[0].html.contains("<strong>Bold</strong>"));
    }

    // ── anchor / reference tests ──────────────────────────────────────────

    #[test]
    fn anchor_marker_stripped_block_stays_in_guide() {
        let body = "Intro.\n\n{#tabla}\n| A | B |\n| --- | --- |\n| 1 | 2 |\n\nOutro.\n";
        let result = render_section_body(body).unwrap();
        assert!(!result.html.contains("{#tabla}"));
        assert!(result.html.contains("<table>"));
    }

    #[test]
    fn slide_ref_inserts_anchored_block() {
        let body =
            "{#tabla}\n| A | B |\n| --- | --- |\n| 1 | 2 |\n\n:::slide\n## T\n\n{{tabla}}\n:::\n";
        let result = render_section_body(body).unwrap();
        assert_eq!(result.slide_html.len(), 1);
        assert!(result.slide_html[0].html.contains("<table>"));
        assert!(!result.slide_html[0].html.contains("{{tabla}}"));
    }

    #[test]
    fn slide_ref_works_when_slide_precedes_anchor() {
        let body = ":::slide\n{{nota}}\n:::\n\n{#nota}\n**Importante.**\n";
        let result = render_section_body(body).unwrap();
        assert!(
            result.slide_html[0]
                .html
                .contains("<strong>Importante.</strong>")
        );
    }

    #[test]
    fn anchored_code_fence_captured_whole_including_blank_lines() {
        let body = "{#code}\n```\nline 1\n\nline 2\n```\n\n:::slide\n{{code}}\n:::\n";
        let result = render_section_body(body).unwrap();
        assert!(result.slide_html[0].html.contains("line 1"));
        assert!(result.slide_html[0].html.contains("line 2"));
        assert!(result.html.contains("line 2"));
    }

    #[test]
    fn unknown_slide_ref_errors() {
        let body = ":::slide\n{{no-existe}}\n:::\n";
        assert!(matches!(
            render_section_body(body),
            Err(Error::UnknownSlideRef(ref n)) if n == "no-existe"
        ));
    }

    #[test]
    fn duplicate_anchor_errors() {
        let body = "{#x}\nuno\n\n{#x}\ndos\n";
        assert!(matches!(
            render_section_body(body),
            Err(Error::DuplicateAnchor(ref n)) if n == "x"
        ));
    }

    #[test]
    fn anchor_without_block_errors() {
        let body = "{#vacio}\n\nTexto.\n";
        assert!(matches!(
            render_section_body(body),
            Err(Error::EmptyAnchor(ref n)) if n == "vacio"
        ));
    }

    #[test]
    fn invalid_anchor_name_is_plain_markdown() {
        let body = "{#con espacio}\nTexto.\n";
        let result = render_section_body(body).unwrap();
        assert!(result.html.contains("{#con espacio}"));
    }

    #[test]
    fn non_ref_braces_in_slide_left_alone() {
        let body = ":::slide\n{{con espacio}}\n:::\n";
        let result = render_section_body(body).unwrap();
        assert!(result.slide_html[0].html.contains("{{con espacio}}"));
    }

    #[test]
    fn render_section_body_no_slides_has_empty_slide_html() {
        let result = render_section_body("Plain text.\n").unwrap();
        assert!(!result.uses_slides);
        assert!(result.slide_html.is_empty());
    }
}
