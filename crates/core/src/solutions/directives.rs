use crate::error::{Error, Result};

/// One top-level segment of a section body: plain Markdown, a hidden solution,
/// a slides-only block, an inline slide (rendered in both guide and slides), a
/// title-only slide, an always-visible admonition, a collapsible extra block,
/// or a guide-only skip block (omitted from slides).
///
/// A directive's payload (`md`) is captured verbatim — it may itself contain
/// nested directives, which the recursive renderers expand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Markdown(String),
    Solution(String),
    Slide {
        md: String,
        light: bool,
    },
    InlineSlide {
        md: String,
        light: bool,
        with_title: bool,
    },
    TitleSlide {
        text: String,
    },
    Warning(String),
    Info(String),
    Extra {
        title: String,
        md: String,
    },
    App(String),
    Skip(String),
    Add {
        md: String,
        visibility: Visibility,
    },
}

/// Where an `:::add` block renders. `Both` is the default; `Slide` and `Both`
/// override an enclosing `:::skip` (the content still reaches the slide).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Both,
    Guide,
    Slide,
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
    InlineSlide { light: bool, with_title: bool },
    TitleSlide { text: String },
    Warning,
    Info,
    Extra { title: String },
    App,
    Skip,
    Add { visibility: Visibility },
}

/// Recognizes a directive opener on its own line. The slide fences
/// (`:::slide`, `:::inline-slide`, `:::title-slide`) take no space and accept
/// an optional `light` modifier (except title-slide); `:::inline-slide` also
/// accepts `with-title`, in any order. Unrecognized modifiers make the line
/// plain Markdown. `::: warning` takes no
/// arguments — with trailing text it is plain Markdown; the same applies to
/// `::: info`. `::: extra` takes an
/// optional title after the keyword. `:::add` takes an optional
/// `visibility=both|guide|slide` argument (default `both`); any other
/// argument makes it plain Markdown.
fn opener(fence: &str) -> Option<Opener> {
    if fence == "::: solucion" {
        Some(Opener::Solution)
    } else if fence == ":::slide" || fence == ":::slide light" {
        Some(Opener::Slide {
            light: fence == ":::slide light",
        })
    } else if let Some(rest) = fence.strip_prefix(":::inline-slide") {
        if !(rest.is_empty() || rest.starts_with(' ')) {
            return None;
        }
        let mut light = false;
        let mut with_title = false;
        for token in rest.split_whitespace() {
            match token {
                "light" if !light => light = true,
                "with-title" if !with_title => with_title = true,
                _ => return None,
            }
        }
        Some(Opener::InlineSlide { light, with_title })
    } else if let Some(rest) = fence.strip_prefix(":::title-slide") {
        (rest.is_empty() || rest.starts_with(' ')).then(|| Opener::TitleSlide {
            text: rest.trim().to_owned(),
        })
    } else if fence == "::: warning" {
        Some(Opener::Warning)
    } else if fence == "::: info" {
        Some(Opener::Info)
    } else if let Some(rest) = fence.strip_prefix("::: extra") {
        (rest.is_empty() || rest.starts_with(' ')).then(|| Opener::Extra {
            title: rest.trim().to_owned(),
        })
    } else if fence == ":::app" {
        Some(Opener::App)
    } else if fence == ":::skip" {
        Some(Opener::Skip)
    } else if let Some(rest) = fence.strip_prefix(":::add") {
        if !(rest.is_empty() || rest.starts_with(' ')) {
            return None;
        }
        let mut visibility = None;
        for token in rest.split_whitespace() {
            let value = token.strip_prefix("visibility=")?;
            if visibility.is_some() {
                return None;
            }
            visibility = Some(match value {
                "both" => Visibility::Both,
                "guide" => Visibility::Guide,
                "slide" => Visibility::Slide,
                _ => return None,
            });
        }
        Some(Opener::Add {
            visibility: visibility.unwrap_or(Visibility::Both),
        })
    } else {
        None
    }
}

/// Recognizes a closing fence on its own line: a bare `:::`, optionally
/// followed by a `#` comment that runs to the end of the line. The comment
/// lets authors label what a closer closes (e.g. `::: # </extra>`); it is
/// ignored entirely.
fn is_closer(fence: &str) -> bool {
    let Some(rest) = fence.strip_prefix(":::") else {
        return false;
    };
    let rest = rest.trim_start();
    rest.is_empty() || rest.starts_with('#')
}

impl Opener {
    fn into_segment(self, md: String) -> Result<Segment> {
        Ok(match self {
            Opener::Solution => Segment::Solution(md),
            Opener::Slide { light } => Segment::Slide { md, light },
            Opener::InlineSlide { light, with_title } => Segment::InlineSlide {
                md,
                light,
                with_title,
            },
            Opener::TitleSlide { text } => {
                if !md.trim().is_empty() {
                    return Err(Error::TitleSlideNotEmpty);
                }
                Segment::TitleSlide { text }
            }
            Opener::Warning => Segment::Warning(md),
            Opener::Info => Segment::Info(md),
            Opener::Extra { title } => Segment::Extra { title, md },
            Opener::App => Segment::App(md),
            Opener::Skip => Segment::Skip(md),
            Opener::Add { visibility } => Segment::Add { md, visibility },
        })
    }

    fn unclosed_error(&self) -> Error {
        match self {
            Opener::Solution => Error::UnclosedSolution,
            Opener::Slide { .. } => Error::UnclosedSlide,
            Opener::InlineSlide { .. } => Error::UnclosedInlineSlide,
            Opener::TitleSlide { .. } => Error::UnclosedTitleSlide,
            Opener::Warning => Error::UnclosedWarning,
            Opener::Info => Error::UnclosedInfo,
            Opener::Extra { .. } => Error::UnclosedExtra,
            Opener::App => Error::UnclosedApp,
            Opener::Skip => Error::UnclosedSkip,
            Opener::Add { .. } => Error::UnclosedAdd,
        }
    }
}

/// Splits a Markdown body into top-level [`Segment`]s. Directive blocks open
/// with their fence (`::: solucion`, `:::slide`, …) and close with a bare
/// `:::`, optionally followed by a `#` comment (`::: # </extra>`). Blocks
/// nest: an opener seen inside a block increments the depth and its line is
/// captured verbatim for the recursive renderer; the matching closer closes
/// the innermost block (depth returns to zero). Fences must sit alone on
/// their line. An unclosed block at end of input is an error.
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
        } else if is_closer(fence) {
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

    // ── warning / info / extra fences ─────────────────────────────────────

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
    fn info_with_arguments_is_plain_markdown() {
        let body = "::: info extra-text\nX.\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Markdown(
                "::: info extra-text\nX.\n:::\n".to_owned()
            )]
        );
    }

    #[test]
    fn unclosed_info_errors() {
        assert!(matches!(
            split_solutions("::: info\nX.\n"),
            Err(Error::UnclosedInfo)
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

    // ── skip fence ────────────────────────────────────────────────────────

    #[test]
    fn skip_block_produces_skip_segment() {
        let body = ":::skip\n  ```yaml\n  Clave: valor\n  ```\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Skip(
                "  ```yaml\n  Clave: valor\n  ```\n".to_owned()
            )]
        );
    }

    #[test]
    fn skip_with_arguments_is_plain_markdown() {
        let body = ":::skip extra\nX.\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Markdown(":::skip extra\nX.\n:::\n".to_owned())]
        );
    }

    #[test]
    fn unclosed_skip_errors() {
        assert!(matches!(
            split_solutions(":::skip\nX.\n"),
            Err(Error::UnclosedSkip)
        ));
    }

    // ── add fence ─────────────────────────────────────────────────────────

    #[test]
    fn add_without_argument_defaults_to_both() {
        let body = ":::add\nX.\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Add {
                md: "X.\n".to_owned(),
                visibility: Visibility::Both,
            }]
        );
    }

    #[test]
    fn add_visibility_values_parse() {
        for (value, visibility) in [
            ("both", Visibility::Both),
            ("guide", Visibility::Guide),
            ("slide", Visibility::Slide),
        ] {
            let body = format!(":::add visibility={value}\nX.\n:::\n");
            let segs = split_solutions(&body).unwrap();
            assert_eq!(
                segs,
                vec![Segment::Add {
                    md: "X.\n".to_owned(),
                    visibility,
                }]
            );
        }
    }

    #[test]
    fn add_unknown_visibility_is_plain_markdown() {
        let body = ":::add visibility=none\nX.\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Markdown(
                ":::add visibility=none\nX.\n:::\n".to_owned()
            )]
        );
    }

    #[test]
    fn add_unknown_argument_is_plain_markdown() {
        let body = ":::add slide\nX.\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Markdown(":::add slide\nX.\n:::\n".to_owned())]
        );
    }

    #[test]
    fn add_duplicate_visibility_is_plain_markdown() {
        let body = ":::add visibility=slide visibility=guide\nX.\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert!(matches!(segs[0], Segment::Markdown(_)));
    }

    #[test]
    fn unclosed_add_errors() {
        assert!(matches!(
            split_solutions(":::add\nX.\n"),
            Err(Error::UnclosedAdd)
        ));
    }

    // ── closer comments ───────────────────────────────────────────────────

    #[test]
    fn closer_with_comment_closes_block() {
        let body = "::: solucion\nAnswer.\n::: # </solucion>\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(segs, vec![Segment::Solution("Answer.\n".to_owned())]);
    }

    #[test]
    fn closer_comment_without_space_also_closes() {
        let body = "::: warning\nX.\n:::# cierre\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(segs, vec![Segment::Warning("X.\n".to_owned())]);
    }

    #[test]
    fn nested_closers_with_comments_close_innermost_first() {
        let body = ":::skip\nA.\n::: extra T\nB.\n::: # </extra>\n::: # </skip>\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Skip(
                "A.\n::: extra T\nB.\n::: # </extra>\n".to_owned()
            )]
        );
    }

    #[test]
    fn closer_comment_outside_block_is_plain_markdown() {
        let body = "Text\n::: # suelto\nMore\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Markdown("Text\n::: # suelto\nMore\n".to_owned())]
        );
    }

    #[test]
    fn closer_with_non_comment_text_does_not_close() {
        assert!(matches!(
            split_solutions("::: warning\nX.\n::: texto\n"),
            Err(Error::UnclosedWarning)
        ));
    }

    #[test]
    fn unclosed_app_errors() {
        assert!(matches!(
            split_solutions(":::app\n<cb-counter></cb-counter>\n"),
            Err(Error::UnclosedApp)
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
                light: false,
                with_title: false,
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
                light: true,
                with_title: false,
            }]
        );
    }

    #[test]
    fn inline_slide_with_title_modifier_sets_flag() {
        let body = ":::inline-slide with-title\nTexto.\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::InlineSlide {
                md: "Texto.\n".to_owned(),
                light: false,
                with_title: true,
            }]
        );
    }

    #[test]
    fn inline_slide_modifiers_combine_in_any_order() {
        for fence in [
            ":::inline-slide light with-title",
            ":::inline-slide with-title light",
        ] {
            let body = format!("{fence}\nTexto.\n:::\n");
            let segs = split_solutions(&body).unwrap();
            assert_eq!(
                segs,
                vec![Segment::InlineSlide {
                    md: "Texto.\n".to_owned(),
                    light: true,
                    with_title: true,
                }]
            );
        }
    }

    #[test]
    fn inline_slide_unknown_modifier_is_plain_markdown() {
        let body = ":::inline-slide dark\nX.\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Markdown(
                ":::inline-slide dark\nX.\n:::\n".to_owned()
            )]
        );
    }

    #[test]
    fn inline_slide_duplicate_modifier_is_plain_markdown() {
        let body = ":::inline-slide light light\nX.\n:::\n";
        let segs = split_solutions(body).unwrap();
        assert_eq!(
            segs,
            vec![Segment::Markdown(
                ":::inline-slide light light\nX.\n:::\n".to_owned()
            )]
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
}
