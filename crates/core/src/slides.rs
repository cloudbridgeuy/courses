//! Slide-only rendering machinery: `{#name}` anchors, `{{name}}` references,
//! and the exercise hero card. Used by [`crate::solutions::render_section_body`]
//! to build slide fragments; none of this affects guide HTML directly.

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::markdown::render_markdown;
use crate::render::escape_html;
use crate::solutions::{Segment, render_solution};

/// An anchored block: plain Markdown, or an exercise (a heading-opened
/// subsection whose `::: solucion` block immediately follows it).
pub(crate) enum Anchor {
    Block(String),
    Exercise { md: String, solution_md: String },
}

/// Collects `{#name}` anchors from Markdown segments, stripping the marker
/// lines while leaving the anchored blocks in place. A heading-opened anchor
/// that runs to the end of its segment and is immediately followed by a
/// `::: solucion` segment becomes an [`Anchor::Exercise`] carrying that
/// solution.
pub(crate) fn extract_anchors(
    segments: Vec<Segment>,
) -> Result<(Vec<Segment>, HashMap<String, Anchor>)> {
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
pub(crate) fn render_slide(md: &str, anchors: &HashMap<String, Anchor>) -> Result<String> {
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use crate::error::Error;
    use crate::solutions::render_section_body;

    #[test]
    fn anchor_marker_stripped_block_stays_in_guide() {
        let body = "Intro.\n\n{#tabla}\n| A | B |\n| --- | --- |\n| 1 | 2 |\n\nOutro.\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(!result.html.contains("{#tabla}"));
        assert!(result.html.contains("<table>"));
    }

    #[test]
    fn slide_ref_inserts_anchored_block() {
        let body =
            "{#tabla}\n| A | B |\n| --- | --- |\n| 1 | 2 |\n\n:::slide\n## T\n\n{{tabla}}\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert_eq!(result.slide_html.len(), 1);
        assert!(result.slide_html[0].html.contains("<table>"));
        assert!(!result.slide_html[0].html.contains("{{tabla}}"));
    }

    #[test]
    fn slide_ref_works_when_slide_precedes_anchor() {
        let body = ":::slide\n{{nota}}\n:::\n\n{#nota}\n**Importante.**\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(
            result.slide_html[0]
                .html
                .contains("<strong>Importante.</strong>")
        );
    }

    #[test]
    fn anchored_code_fence_captured_whole_including_blank_lines() {
        let body = "{#code}\n```\nline 1\n\nline 2\n```\n\n:::slide\n{{code}}\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.slide_html[0].html.contains("line 1"));
        assert!(result.slide_html[0].html.contains("line 2"));
        assert!(result.html.contains("line 2"));
    }

    #[test]
    fn unknown_slide_ref_errors() {
        let body = ":::slide\n{{no-existe}}\n:::\n";
        assert!(matches!(
            render_section_body("Sección", body),
            Err(Error::UnknownSlideRef(ref n)) if n == "no-existe"
        ));
    }

    #[test]
    fn duplicate_anchor_errors() {
        let body = "{#x}\nuno\n\n{#x}\ndos\n";
        assert!(matches!(
            render_section_body("Sección", body),
            Err(Error::DuplicateAnchor(ref n)) if n == "x"
        ));
    }

    #[test]
    fn anchor_without_block_errors() {
        let body = "{#vacio}\n\nTexto.\n";
        assert!(matches!(
            render_section_body("Sección", body),
            Err(Error::EmptyAnchor(ref n)) if n == "vacio"
        ));
    }

    #[test]
    fn invalid_anchor_name_is_plain_markdown() {
        let body = "{#con espacio}\nTexto.\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.html.contains("{#con espacio}"));
    }

    #[test]
    fn non_ref_braces_in_slide_left_alone() {
        let body = ":::slide\n{{con espacio}}\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.slide_html[0].html.contains("{{con espacio}}"));
    }

    #[test]
    fn heading_anchor_captures_subsection_until_next_heading() {
        let body = "{#ej-1}\n### Ejercicio 1\n\nEnunciado uno.\n\nMás texto.\n\n### Otra sección\n\nFuera.\n\n:::slide\n{{ej-1}}\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
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
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.slide_html[0].html.contains("Enunciado."));
        assert!(!result.slide_html[0].html.contains("Después."));
    }

    #[test]
    fn exercise_anchor_renders_hero_card_with_solution_toggle() {
        let body = "{#ej}\n### Ejercicio 1 — Cree su repositorio\n\nEnunciado.\n\n::: solucion\nRespuesta secreta.\n:::\n\n:::slide\n{{ej}}\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
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
        let result = render_section_body("Sección", body).unwrap();
        let slide = &result.slide_html[0].html;
        assert!(!slide.contains("cb-ejercicio-kicker"));
        assert!(slide.contains("<h3>Reto final</h3>"));
    }

    #[test]
    fn heading_anchor_not_followed_by_solution_stays_plain_block() {
        let body = "{#sec}\n### Sección\n\nTexto.\n\n### Otra\n\nX.\n\n:::slide\n{{sec}}\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        let slide = &result.slide_html[0].html;
        assert!(!slide.contains("cb-ejercicio"));
        assert!(slide.contains("Texto."));
    }

    #[test]
    fn warning_between_anchor_and_solution_prevents_exercise_pairing() {
        // Warning hits `other =>`, resetting tail_anchor — no Anchor::Exercise.
        let body = "{#ej}\n### Ej\n\nX.\n\n::: warning\nW.\n:::\n::: solucion\nR.\n:::\n\n:::slide\n{{ej}}\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(!result.slide_html[0].html.contains("cb-ejercicio"));
        assert!(result.html.contains("R."));
    }
}
