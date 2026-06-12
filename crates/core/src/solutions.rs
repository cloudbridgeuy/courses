use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::markdown::render_markdown;
use crate::render::escape_html;

/// One segment of a section body: plain Markdown, a hidden solution, a slide,
/// an always-visible warning admonition, or a collapsible extra block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Markdown(String),
    Solution(String),
    Slide { md: String, light: bool },
    Warning(String),
    Extra { title: String, md: String },
}

/// A rendered slide HTML fragment with its display variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideFragment {
    pub html: String,
    pub light: bool,
}

/// Splits a Markdown body on `::: solucion`, `:::slide`, `::: warning`, and
/// `::: extra <title>` fenced blocks (closed by `:::`).
///
/// Fences must sit alone on their line. Nesting is rejected; an unclosed
/// block is rejected.
///
/// Note: `":::slide"` has no space; the other fences use a space.
/// `"::: warning"` takes no arguments — with trailing text it is plain
/// Markdown. `"::: extra"` takes an optional title after the keyword.
pub fn split_solutions(body: &str) -> Result<Vec<Segment>> {
    enum State {
        Outside,
        InSolution,
        InSlide { light: bool },
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
/// toggle markup (button plus hidden panel). `:::slide` blocks are collected
/// separately and omitted from the guide HTML. `::: warning` renders as an
/// always-visible admonition div; `::: extra <title>` renders as a closed
/// `<details>` block — both are guide-only (not included in slides).
///
/// A line `{#name}` in plain Markdown anchors the block that follows (until a
/// blank line; a whole code fence; or, when the block opens with a heading,
/// the whole subsection up to the next heading, `---` rule, or `::: solucion`
/// block). The marker line is
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
                slide_html.push(SlideFragment {
                    html: render_slide(&md, &anchors)?,
                    light,
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

/// An anchored block: plain Markdown, or an exercise (a heading-opened
/// subsection whose `::: solucion` block immediately follows it).
enum Anchor {
    Block(String),
    Exercise { md: String, solution_md: String },
}

/// Collects `{#name}` anchors from Markdown segments, stripping the marker
/// lines while leaving the anchored blocks in place. A heading-opened anchor
/// that runs to the end of its segment and is immediately followed by a
/// `::: solucion` segment becomes an [`Anchor::Exercise`] carrying that
/// solution.
fn extract_anchors(segments: Vec<Segment>) -> Result<(Vec<Segment>, HashMap<String, Anchor>)> {
    let mut anchors = HashMap::new();
    let mut out: Vec<Segment> = Vec::with_capacity(segments.len());
    let mut tail_anchor: Option<String> = None;
    for segment in segments {
        match segment {
            Segment::Markdown(md) => {
                let (stripped, tail) = strip_anchors(&md, &mut anchors)?;
                tail_anchor = tail;
                out.push(Segment::Markdown(stripped));
            }
            Segment::Solution(solution) => {
                if let Some(name) = tail_anchor.take()
                    && let Some(Anchor::Block(md)) = anchors.remove(&name)
                {
                    anchors.insert(
                        name,
                        Anchor::Exercise {
                            md,
                            solution_md: solution.clone(),
                        },
                    );
                }
                out.push(Segment::Solution(solution));
            }
            other => {
                tail_anchor = None;
                out.push(other);
            }
        }
    }
    Ok((out, anchors))
}

/// Strips anchor markers from one Markdown segment. Also returns the name of
/// a heading-opened anchor whose block ran to the end of the segment, if any —
/// the candidate for exercise-solution attachment.
fn strip_anchors(
    md: &str,
    anchors: &mut HashMap<String, Anchor>,
) -> Result<(String, Option<String>)> {
    let lines: Vec<&str> = md.lines().collect();
    let mut out = String::new();
    let mut tail_anchor = None;
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
        if anchors
            .insert(name.to_owned(), Anchor::Block(block.clone()))
            .is_some()
        {
            return Err(Error::DuplicateAnchor(name.to_owned()));
        }
        let is_heading = lines[start].starts_with('#');
        let trailing = lines[end..].iter().all(|l| l.trim().is_empty());
        tail_anchor = (is_heading && trailing).then(|| name.to_owned());
        out.push_str(&block);
        i = end;
    }
    Ok((out, tail_anchor))
}

/// End (exclusive) of the block starting at `start`: a whole code fence, a
/// whole subsection when the block opens with a heading (until the next
/// heading, a `---` rule, or the end of the segment — a `::: solucion` fence
/// already ends the segment, so solutions are never captured), or otherwise
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
    if lines.get(start).is_some_and(|l| l.starts_with('#')) {
        let mut end = start + 1;
        while end < lines.len() && !lines[end].starts_with('#') && lines[end].trim() != "---" {
            end += 1;
        }
        return end;
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

/// Renders slide Markdown to HTML, expanding `{{name}}` lines: block anchors
/// are inlined as Markdown; exercise anchors render as a hero card with a
/// solution toggle.
fn render_slide(md: &str, anchors: &HashMap<String, Anchor>) -> Result<String> {
    let mut html = String::new();
    let mut buf = String::new();
    for line in md.lines() {
        let reference = line
            .trim()
            .strip_prefix("{{")
            .and_then(|r| r.strip_suffix("}}"))
            .map(str::trim)
            .filter(|name| is_valid_anchor_name(name));
        let Some(name) = reference else {
            buf.push_str(line);
            buf.push('\n');
            continue;
        };
        match anchors.get(name) {
            None => return Err(Error::UnknownSlideRef(name.to_owned())),
            Some(Anchor::Block(block)) => buf.push_str(block),
            Some(Anchor::Exercise { md, solution_md }) => {
                if !buf.is_empty() {
                    html.push_str(&render_markdown(&std::mem::take(&mut buf)));
                }
                html.push_str(&render_exercise(md, solution_md));
            }
        }
    }
    if !buf.is_empty() {
        html.push_str(&render_markdown(&buf));
    }
    Ok(html)
}

/// Renders an exercise hero card: a kicker (the part of the heading before
/// " — ", e.g. "Ejercicio 1"), the title, the statement, and the standard
/// solution toggle.
fn render_exercise(md: &str, solution_md: &str) -> String {
    let lines: Vec<&str> = md.lines().collect();
    let heading_idx = lines.iter().position(|l| l.starts_with('#'));
    let heading = heading_idx
        .map(|i| lines[i].trim_start_matches('#').trim())
        .unwrap_or_default();
    let statement_md = heading_idx
        .map(|i| lines[i + 1..].join("\n"))
        .unwrap_or_else(|| md.to_owned());
    let (kicker, title) = match heading.split_once(" — ") {
        Some((kicker, title)) => (kicker, title),
        None => ("", heading),
    };
    let kicker_html = if kicker.is_empty() {
        String::new()
    } else {
        format!(
            "<p class=\"cb-ejercicio-kicker\">{}</p>\n",
            escape_html(kicker)
        )
    };
    format!(
        "<div class=\"cb-ejercicio\">\n\
         {kicker_html}<h3>{}</h3>\n\
         <div class=\"cb-ejercicio-cuerpo\">\n{}</div>\n\
         {}</div>\n",
        escape_html(title),
        render_markdown(&statement_md),
        render_solution(&render_markdown(solution_md)),
    )
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
    fn heading_anchor_captures_subsection_until_next_heading() {
        let body = "{#ej-1}\n### Ejercicio 1\n\nEnunciado uno.\n\nMás texto.\n\n### Otra sección\n\nFuera.\n\n:::slide\n{{ej-1}}\n:::\n";
        let result = render_section_body(body).unwrap();
        let slide = &result.slide_html[0].html;
        assert!(slide.contains("Ejercicio 1"));
        assert!(slide.contains("Enunciado uno."));
        assert!(slide.contains("Más texto."));
        assert!(!slide.contains("Otra sección"));
        assert!(!slide.contains("Fuera."));
    }

    #[test]
    fn heading_anchor_stops_at_thematic_break() {
        let body =
            "{#ej}\n### Ejercicio\n\nEnunciado.\n\n---\n\nDespués.\n\n:::slide\n{{ej}}\n:::\n";
        let result = render_section_body(body).unwrap();
        assert!(result.slide_html[0].html.contains("Enunciado."));
        assert!(!result.slide_html[0].html.contains("Después."));
    }

    #[test]
    fn exercise_anchor_renders_hero_card_with_solution_toggle() {
        let body = "{#ej}\n### Ejercicio 1 — Cree su repositorio\n\nEnunciado.\n\n::: solucion\nRespuesta secreta.\n:::\n\n:::slide\n{{ej}}\n:::\n";
        let result = render_section_body(body).unwrap();
        let slide = &result.slide_html[0].html;
        assert!(slide.contains("class=\"cb-ejercicio\""));
        assert!(slide.contains("class=\"cb-ejercicio-kicker\">Ejercicio 1</p>"));
        assert!(slide.contains("<h3>Cree su repositorio</h3>"));
        assert!(slide.contains("Enunciado."));
        assert!(slide.contains("Ver solución"));
        assert!(slide.contains("Respuesta secreta."));
        assert!(slide.contains("hidden"));
        // guide still renders its own toggle
        assert!(result.html.contains("Respuesta secreta."));
    }

    #[test]
    fn exercise_heading_without_dash_has_no_kicker() {
        let body = "{#ej}\n### Reto final\n\nEnunciado.\n\n::: solucion\nR.\n:::\n\n:::slide\n{{ej}}\n:::\n";
        let result = render_section_body(body).unwrap();
        let slide = &result.slide_html[0].html;
        assert!(!slide.contains("cb-ejercicio-kicker"));
        assert!(slide.contains("<h3>Reto final</h3>"));
    }

    #[test]
    fn heading_anchor_not_followed_by_solution_stays_plain_block() {
        let body = "{#sec}\n### Sección\n\nTexto.\n\n### Otra\n\nX.\n\n:::slide\n{{sec}}\n:::\n";
        let result = render_section_body(body).unwrap();
        let slide = &result.slide_html[0].html;
        assert!(!slide.contains("cb-ejercicio"));
        assert!(slide.contains("Texto."));
    }

    #[test]
    fn warning_between_anchor_and_solution_prevents_exercise_pairing() {
        // Warning hits `other =>`, resetting tail_anchor — no Anchor::Exercise.
        let body = "{#ej}\n### Ej\n\nX.\n\n::: warning\nW.\n:::\n::: solucion\nR.\n:::\n\n:::slide\n{{ej}}\n:::\n";
        let result = render_section_body(body).unwrap();
        assert!(!result.slide_html[0].html.contains("cb-ejercicio"));
        assert!(result.html.contains("R."));
    }

    #[test]
    fn render_section_body_no_slides_has_empty_slide_html() {
        let result = render_section_body("Plain text.\n").unwrap();
        assert!(!result.uses_slides);
        assert!(result.slide_html.is_empty());
    }

    // ── warning / extra rendering ─────────────────────────────────────────

    #[test]
    fn warning_renders_admonition_div_with_inner_markdown() {
        let body = "::: warning\n**Atención**: [aviso](https://example.com).\n:::\n";
        let result = render_section_body(body).unwrap();
        assert!(result.html.contains("<div class=\"cb-warning\">"));
        assert!(result.html.contains("<strong>Atención</strong>"));
        assert!(result.html.contains("<a href=\"https://example.com\">"));
    }

    #[test]
    fn extra_renders_closed_details_with_summary_title() {
        let body = "::: extra ¿Qué es un remoto?\nUn remoto es…\n:::\n";
        let result = render_section_body(body).unwrap();
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
        let result = render_section_body(body).unwrap();
        assert!(
            result
                .html
                .contains("<summary>Contenido adicional</summary>")
        );
    }

    #[test]
    fn extra_title_is_html_escaped() {
        let body = "::: extra Uso de <code> & otros\nX.\n:::\n";
        let result = render_section_body(body).unwrap();
        assert!(
            result
                .html
                .contains("<summary>Uso de &lt;code&gt; &amp; otros</summary>")
        );
    }

    #[test]
    fn warning_and_extra_omitted_from_slides_and_do_not_set_flags() {
        let body = "::: warning\nW.\n:::\n::: extra T\nE.\n:::\n";
        let result = render_section_body(body).unwrap();
        assert!(!result.uses_solutions);
        assert!(!result.uses_slides);
        assert!(result.slide_html.is_empty());
    }
}
