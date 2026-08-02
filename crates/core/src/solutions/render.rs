use super::directives::{Segment, SlideFragment, Visibility, split_solutions};
use crate::error::{Error, Result};
use crate::markdown::render_markdown;
use crate::render::escape_html;
use crate::slides::{extract_anchors, render_slide_content};

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
/// `::: solucion`, `::: warning`, `::: info`, and `::: extra` produce their guide markup
/// and nest freely (an info block inside a solution, an extra inside a warning,
/// etc.). `:::slide` blocks are collected separately and omitted from the
/// guide HTML. `:::inline-slide` renders in BOTH places; with the
/// `with-title` modifier its slide copy is prefixed with the nearest heading
/// found in the top-level Markdown above it (falling back to the section
/// `title`) — the guide copy is untouched, since that heading already sits
/// above it in the document. `:::title-slide`
/// produces a slide showing only `title` (the section heading); its body must
/// be empty. `:::skip` renders its content in the guide only — inside a slide
/// (including an inline slide's slide copy) it is dropped. `:::add` filters
/// by `visibility` (`both` default, `guide`, `slide`); with `both` or `slide`
/// it escapes an enclosing `:::skip` and still reaches the slide. Slide-family
/// directives are top-level only — nesting one inside
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
    let mut last_heading: Option<String> = None;
    for segment in segments {
        match segment {
            Segment::Markdown(md) => {
                if let Some(heading) = last_heading_line(&md) {
                    last_heading = Some(heading);
                }
                html.push_str(&render_markdown(&md));
            }
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
            Segment::InlineSlide {
                md,
                light,
                with_title,
            } => {
                uses_slides = true;
                let slide_md = if with_title {
                    let heading = last_heading
                        .clone()
                        .unwrap_or_else(|| format!("## {title}"));
                    format!("{heading}\n\n{md}")
                } else {
                    md.clone()
                };
                slide_html.push(SlideFragment {
                    html: render_slide_content(&slide_md, &anchors)?,
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
            Segment::Info(md) => {
                let inner = render_guide_segments(&md, &mut uses_solutions)?;
                html.push_str(&render_info(&inner));
            }
            Segment::Extra { title, md } => {
                let inner = render_guide_segments(&md, &mut uses_solutions)?;
                html.push_str(&render_extra(&title, &inner));
            }
            Segment::App(md) => {
                html.push_str(&render_app(&md));
            }
            Segment::Skip(md) => {
                html.push_str(&render_guide_segments(&md, &mut uses_solutions)?);
            }
            Segment::Add { md, visibility } => {
                if visibility != Visibility::Slide {
                    html.push_str(&render_guide_segments(&md, &mut uses_solutions)?);
                }
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
/// `::: solucion`, `::: warning`, `::: info`, and `::: extra` produce their guide markup;
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
            Segment::Info(inner) => {
                let body = render_guide_segments(&inner, uses_solutions)?;
                html.push_str(&render_info(&body));
            }
            Segment::Extra { title, md } => {
                let body = render_guide_segments(&md, uses_solutions)?;
                html.push_str(&render_extra(&title, &body));
            }
            Segment::App(md) => {
                html.push_str(&render_app(&md));
            }
            Segment::Skip(inner) => {
                html.push_str(&render_guide_segments(&inner, uses_solutions)?);
            }
            Segment::Add { md, visibility } => {
                if visibility != Visibility::Slide {
                    html.push_str(&render_guide_segments(&md, uses_solutions)?);
                }
            }
            Segment::Slide { .. } => return Err(Error::NestedSlide),
            Segment::InlineSlide { .. } => return Err(Error::NestedInlineSlide),
            Segment::TitleSlide { .. } => return Err(Error::NestedTitleSlide),
        }
    }
    Ok(html)
}

/// Returns the last ATX heading line in `md` (trimmed, Markdown kept
/// verbatim), skipping lines inside fenced code blocks. Used by
/// `:::inline-slide with-title` to pick up the heading that precedes it.
fn last_heading_line(md: &str) -> Option<String> {
    let mut in_fence = false;
    let mut last = None;
    for line in md.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
            last = Some(trimmed.to_owned());
        }
    }
    last
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

/// Renders an always-visible information admonition. The symbol comes from CSS
/// (`.cb-info::before`), keeping markup minimal.
pub(crate) fn render_info(inner_html: &str) -> String {
    format!("<div class=\"cb-info\">\n{inner_html}</div>\n")
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

    // ── app fence rendering ────────────────────────────────────────────────

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

    // ── skip fence rendering ──────────────────────────────────────────────

    #[test]
    fn skip_inside_inline_slide_renders_in_guide_but_not_slide() {
        let body = ":::inline-slide\nVisible.\n:::skip\n```yaml\nClave: valor\n```\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.html.contains("Visible."));
        assert!(result.html.contains("Clave"));
        assert!(result.slide_html[0].html.contains("Visible."));
        assert!(!result.slide_html[0].html.contains("Clave"));
    }

    #[test]
    fn skip_at_top_level_renders_in_guide_only() {
        let body = ":::skip\nSolo guía.\n:::\n:::slide\nDiapositiva.\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.html.contains("Solo guía."));
        assert!(!result.slide_html[0].html.contains("Solo guía."));
    }

    #[test]
    fn skip_inside_slide_renders_nothing() {
        let body = ":::slide\nVisible.\n:::skip\nOculto.\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.slide_html[0].html.contains("Visible."));
        assert!(!result.slide_html[0].html.contains("Oculto."));
        assert!(!result.html.contains("Oculto."));
    }

    #[test]
    fn solution_inside_skip_still_sets_uses_solutions() {
        let body = ":::skip\n::: solucion\nRespuesta.\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.uses_solutions);
        assert!(result.html.contains("Respuesta."));
    }

    // ── add fence rendering ───────────────────────────────────────────────

    #[test]
    fn add_default_renders_in_guide_and_slide() {
        let body = ":::inline-slide\nVisible.\n:::add\nExtra.\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.html.contains("Extra."));
        assert!(result.slide_html[0].html.contains("Extra."));
    }

    #[test]
    fn add_visibility_slide_renders_in_slide_only() {
        let body =
            ":::inline-slide\nVisible.\n:::add visibility=slide\nSolo diapositiva.\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(!result.html.contains("Solo diapositiva."));
        assert!(result.slide_html[0].html.contains("Solo diapositiva."));
    }

    #[test]
    fn add_visibility_guide_renders_in_guide_only() {
        let body = ":::inline-slide\nVisible.\n:::add visibility=guide\nSolo guía.\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.html.contains("Solo guía."));
        assert!(!result.slide_html[0].html.contains("Solo guía."));
    }

    #[test]
    fn add_inside_skip_overrides_skip_for_slide() {
        let body = ":::inline-slide\n:::skip\nOculto.\n:::add\nRescatado.\n:::\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.html.contains("Oculto."));
        assert!(result.html.contains("Rescatado."));
        assert!(!result.slide_html[0].html.contains("Oculto."));
        assert!(result.slide_html[0].html.contains("Rescatado."));
    }

    #[test]
    fn add_visibility_slide_inside_skip_renders_in_slide_only() {
        let body = ":::inline-slide\n:::skip\nOculto.\n:::add visibility=slide\nRescatado.\n:::\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(!result.html.contains("Rescatado."));
        assert!(result.slide_html[0].html.contains("Rescatado."));
    }

    #[test]
    fn add_visibility_guide_inside_skip_stays_out_of_slide() {
        let body = ":::inline-slide\n:::skip\n:::add visibility=guide\nSolo guía.\n:::\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.html.contains("Solo guía."));
        assert!(!result.slide_html[0].html.contains("Solo guía."));
    }

    #[test]
    fn add_inside_nested_skip_still_escapes_to_slide() {
        let body = ":::inline-slide\n:::skip\n:::skip\n:::add\nProfundo.\n:::\n:::\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.html.contains("Profundo."));
        assert!(result.slide_html[0].html.contains("Profundo."));
    }

    #[test]
    fn add_at_top_level_visibility_slide_renders_nowhere() {
        // Top-level content only reaches slides through slide directives, so
        // a slide-only :::add outside one has no slide to land in.
        let body = ":::add visibility=slide\nX.\n:::\n:::slide\nDiapositiva.\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(!result.html.contains("X."));
        assert!(!result.slide_html[0].html.contains("X."));
    }

    #[test]
    fn solution_inside_add_sets_uses_solutions() {
        let body = ":::add\n::: solucion\nRespuesta.\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.uses_solutions);
        assert!(result.html.contains("Respuesta."));
    }

    // ── inline-slide with-title ───────────────────────────────────────────

    #[test]
    fn inline_slide_with_title_prepends_nearest_heading_to_slide_only() {
        let body = "## Funciones\n\nProsa.\n:::inline-slide with-title\n- **`!Ref`**\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert_eq!(result.slide_html.len(), 1);
        assert!(result.slide_html[0].html.contains("Funciones"));
        // Guide keeps the heading once (from the flowing Markdown, not the copy).
        assert_eq!(result.html.matches("Funciones").count(), 1);
    }

    #[test]
    fn inline_slide_with_title_uses_nearest_of_several_headings() {
        let body = "## Primero\nA.\n### Segundo\nB.\n:::inline-slide with-title\nX.\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.slide_html[0].html.contains("Segundo"));
        assert!(!result.slide_html[0].html.contains("Primero"));
    }

    #[test]
    fn inline_slide_with_title_ignores_headings_in_code_fences() {
        let body = "## Real\n\n```\n# comentario\n```\n:::inline-slide with-title\nX.\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.slide_html[0].html.contains("Real"));
        assert!(!result.slide_html[0].html.contains("comentario</h1>"));
    }

    #[test]
    fn inline_slide_with_title_falls_back_to_section_title() {
        let body = "Prosa sin encabezado.\n:::inline-slide with-title\nX.\n:::\n";
        let result = render_section_body("Mi sección", body).unwrap();
        assert!(result.slide_html[0].html.contains("Mi sección"));
        assert!(!result.html.contains("Mi sección"));
    }

    #[test]
    fn inline_slide_without_with_title_gets_no_heading() {
        let body = "## Funciones\nProsa.\n:::inline-slide\nX.\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(!result.slide_html[0].html.contains("Funciones"));
    }

    #[test]
    fn inline_slide_with_title_light_keeps_light_variant() {
        let body = "## T\n:::inline-slide light with-title\nX.\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.slide_html[0].light);
        assert!(result.slide_html[0].html.contains("<h2 id=\"t\">"));
    }

    #[test]
    fn last_heading_line_picks_last_and_skips_fences() {
        assert_eq!(
            last_heading_line("## A\ntext\n### B\n"),
            Some("### B".to_owned())
        );
        assert_eq!(last_heading_line("```\n# in fence\n```\n"), None);
        assert_eq!(last_heading_line("#not-a-heading\ntext\n"), None);
        assert_eq!(last_heading_line("plain text\n"), None);
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
    fn info_inside_solution_renders_both() {
        let body = "::: solucion\nRespuesta.\n::: info\nContexto.\n:::\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.uses_solutions);
        assert!(result.html.contains("class=\"solucion\""));
        assert!(result.html.contains("<div class=\"cb-info\">"));
        assert!(result.html.contains("Contexto."));
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
    fn info_renders_admonition_div_with_inner_markdown() {
        let body = "::: info\n**Nota**: [más información](https://example.com).\n:::\n";
        let result = render_section_body("Sección", body).unwrap();
        assert!(result.html.contains("<div class=\"cb-info\">"));
        assert!(result.html.contains("<strong>Nota</strong>"));
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
    fn admonitions_and_extra_omitted_from_slides_and_do_not_set_flags() {
        let body = "::: warning\nW.\n:::\n::: info\nI.\n:::\n::: extra T\nE.\n:::\n";
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
