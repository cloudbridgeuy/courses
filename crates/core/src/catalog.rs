use crate::assets::{
    APPS_JS_PATH, MERMAID_INIT_JS_PATH, MERMAID_JS_PATH, PageAssets, SHIKI_INIT_JS_PATH,
    TOGGLE_JS_PATH,
};
use crate::course::{Course, CourseSlug, GuideSection, Session, SessionSlug};
use crate::error::{Error, Result};
use crate::goto_apps::{collect_ids, expand_goto_apps};
use crate::manifest::{SessionEntry, parse_manifest};
use crate::section::{parse_frontmatter, split_frontmatter};
use crate::solutions::{SlideFragment, render_section_body};

/// Raw inputs for one course: its slug, its manifest TOML, and its section
/// files as `(file name, contents)` pairs.
#[derive(Debug, Clone, Copy)]
pub struct CourseInput<'a> {
    pub slug: &'a str,
    pub manifest: &'a str,
    pub files: &'a [(String, String)],
}

/// A fully parsed course, plus one [`PageAssets`] per session, index-aligned
/// with `course.sessions` (built together in [`parse_course`], so the lengths
/// always match).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedCourse {
    pub course: Course,
    pub session_assets: Vec<PageAssets>,
    /// Rendered slide fragments per session, in section order.
    /// Index-aligned with `course.sessions`.
    pub session_slides: Vec<Vec<SlideFragment>>,
}

/// Parses raw course inputs into a [`LoadedCourse`] (parse, don't validate).
///
/// Each session's assets carry the base stylesheet, the toggle script when any
/// of that session's sections uses a `::: solucion` block, and the frontmatter
/// extras of its sections, merged and deduped in section order.
pub fn parse_course(input: &CourseInput<'_>) -> Result<LoadedCourse> {
    let slug = CourseSlug::parse(input.slug)?;
    let manifest = parse_manifest(input.manifest)?;

    let mut sessions = Vec::with_capacity(manifest.sessions.len());
    let mut session_assets = Vec::with_capacity(manifest.sessions.len());
    let mut session_slides = Vec::with_capacity(manifest.sessions.len());

    for entry in &manifest.sessions {
        let (session, assets, slides) = assemble_session(entry, input.files)?;
        sessions.push(session);
        session_assets.push(assets);
        session_slides.push(slides);
    }

    Ok(LoadedCourse {
        course: Course {
            slug,
            title: manifest.title,
            sessions,
        },
        session_assets,
        session_slides,
    })
}

/// Assembles one session, its page assets, and its slide fragments from its manifest entry.
fn assemble_session(
    entry: &SessionEntry,
    files: &[(String, String)],
) -> Result<(Session, PageAssets, Vec<SlideFragment>)> {
    let session_slug = SessionSlug::parse(&entry.slug)?;
    let mut assets = PageAssets::base();
    let mut sections = Vec::with_capacity(entry.sections.len());
    let mut uses_solutions = false;
    let mut uses_mermaid = false;
    let mut uses_syntax_highlighting = false;
    let mut uses_apps = false;
    let mut uses_file_apps = false;
    let mut all_slides = Vec::new();
    // Source file of each section / slide, index-aligned, so a cb-goto error
    // after the loop can still name the file it came from.
    let mut section_files = Vec::with_capacity(entry.sections.len());
    let mut slide_files = Vec::new();

    for file in &entry.sections {
        let contents = files
            .iter()
            .find(|(name, _)| name == file)
            .map(|(_, contents)| contents.as_str())
            .ok_or_else(|| Error::MissingSection(file.clone()))?;

        let (raw_frontmatter, body) = split_frontmatter(contents).map_err(|e| in_file(file, &e))?;
        let frontmatter = parse_frontmatter(raw_frontmatter).map_err(|e| in_file(file, &e))?;
        let rendered =
            render_section_body(&frontmatter.title, body).map_err(|e| in_file(file, &e))?;

        uses_solutions |= rendered.uses_solutions;
        uses_mermaid |= rendered.uses_mermaid;
        uses_syntax_highlighting |= rendered.html.contains("<code class=\"language-")
            || rendered.html.contains("<cb-file")
            || rendered.html.contains("<cb-http")
            || rendered.slide_html.iter().any(|slide| {
                slide.html.contains("<code class=\"language-")
                    || slide.html.contains("<cb-file")
                    || slide.html.contains("<cb-http")
            });
        uses_file_apps |= rendered.html.contains("<cb-file")
            || rendered
                .slide_html
                .iter()
                .any(|slide| slide.html.contains("<cb-file"));
        uses_apps |= rendered.uses_apps;
        section_files.push(file.as_str());
        slide_files.extend(std::iter::repeat_n(
            file.as_str(),
            rendered.slide_html.len(),
        ));
        all_slides.extend(rendered.slide_html);

        for href in &frontmatter.styles {
            assets.push_style(href);
        }
        for src in &frontmatter.scripts {
            assets.push_script(src);
        }
        sections.push(GuideSection {
            title: frontmatter.title,
            body_html: rendered.html,
        });
    }

    // cb-goto targets resolve against every heading id in the session's guide
    // copy, so a button may point across section files. This has to run after
    // the section loop: only then is the full id set known.
    let mut ids = std::collections::HashSet::new();
    for section in &sections {
        collect_ids(&section.body_html, &mut ids);
    }
    for (section, file) in sections.iter_mut().zip(section_files.iter().copied()) {
        section.body_html =
            expand_goto_apps(&section.body_html, &ids).map_err(|e| in_file(file, &e))?;
    }
    for (slide, file) in all_slides.iter_mut().zip(slide_files.iter().copied()) {
        slide.html = expand_goto_apps(&slide.html, &ids).map_err(|e| in_file(file, &e))?;
    }

    if uses_solutions || uses_file_apps {
        assets.push_script(TOGGLE_JS_PATH);
    }
    if uses_mermaid {
        assets.push_script(MERMAID_JS_PATH);
        assets.push_script(MERMAID_INIT_JS_PATH);
    }
    if uses_apps {
        assets.push_script(APPS_JS_PATH);
    }
    // `cb-file` creates its code block when apps.js runs, so apps must be
    // registered before Shiki scans the document for language-tagged blocks.
    // `cb-http` needs Shiki too — its response panel highlights on demand
    // through the `window.cbShiki` helper shiki-init.js exposes.
    if uses_syntax_highlighting {
        assets.push_script(SHIKI_INIT_JS_PATH);
    }

    Ok((
        Session {
            slug: session_slug,
            title: entry.title.clone(),
            sections,
        },
        assets,
        all_slides,
    ))
}

fn in_file(file: &str, error: &Error) -> Error {
    Error::InvalidSection {
        file: file.to_owned(),
        message: error.to_string(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::assets::{APPS_JS_PATH, SHIKI_INIT_JS_PATH, TOGGLE_JS_PATH};

    fn make_section(title: &str, body: &str) -> String {
        format!("+++\ntitle = \"{title}\"\n+++\n{body}")
    }

    fn make_files(pairs: &[(&str, String)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(name, content)| ((*name).to_owned(), content.clone()))
            .collect()
    }

    fn make_manifest_one_session(title: &str, sections: &[&str]) -> String {
        let list = sections
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "title = \"{title}\"\n\n[[session]]\nslug = \"s1\"\ntitle = \"Session 1\"\nsections = [{list}]\n"
        )
    }

    #[test]
    fn happy_path_two_sections_in_manifest_order() {
        let files = make_files(&[
            ("01-a.md", make_section("Section A", "Body A\n")),
            ("02-b.md", make_section("Section B", "Body B\n")),
        ]);
        let input = CourseInput {
            slug: "my-course",
            manifest: &make_manifest_one_session("My Course", &["01-a.md", "02-b.md"]),
            files: &files,
        };
        let loaded = parse_course(&input).unwrap();
        assert_eq!(loaded.course.title, "My Course");
        assert_eq!(loaded.course.sessions[0].sections.len(), 2);
        assert_eq!(loaded.course.sessions[0].sections[0].title, "Section A");
        assert_eq!(loaded.course.sessions[0].sections[1].title, "Section B");
        // bodies are rendered HTML
        assert!(
            loaded.course.sessions[0].sections[0]
                .body_html
                .contains("Body A")
        );
        assert!(
            loaded.course.sessions[0].sections[1]
                .body_html
                .contains("Body B")
        );
        assert!(loaded.session_slides[0].is_empty());
    }

    #[test]
    fn section_with_solution_injects_toggle_script() {
        let body = "Before\n::: solucion\nAnswer\n:::\nAfter\n";
        let files = make_files(&[("01-sol.md", make_section("With Sol", body))]);
        let input = CourseInput {
            slug: "course-a",
            manifest: &make_manifest_one_session("Course A", &["01-sol.md"]),
            files: &files,
        };
        let loaded = parse_course(&input).unwrap();
        assert!(
            loaded.session_assets[0]
                .scripts
                .contains(&TOGGLE_JS_PATH.to_owned())
        );
        assert!(loaded.session_slides[0].is_empty());
    }

    #[test]
    fn section_without_solution_does_not_inject_toggle_script() {
        let files = make_files(&[("01-plain.md", make_section("Plain", "No solutions.\n"))]);
        let input = CourseInput {
            slug: "course-b",
            manifest: &make_manifest_one_session("Course B", &["01-plain.md"]),
            files: &files,
        };
        let loaded = parse_course(&input).unwrap();
        assert!(
            !loaded.session_assets[0]
                .scripts
                .contains(&TOGGLE_JS_PATH.to_owned())
        );
    }

    #[test]
    fn file_app_injects_toggle_apps_and_shiki_scripts() {
        let body = ":::app\n<cb-file path=\"./sample.yml\" type=\"yaml\"></cb-file>\n:::\n";
        let files = make_files(&[("01-file.md", make_section("File", body))]);
        let input = CourseInput {
            slug: "course-file",
            manifest: &make_manifest_one_session("Course File", &["01-file.md"]),
            files: &files,
        };
        let loaded = parse_course(&input).unwrap();
        assert_eq!(
            loaded.session_assets[0].scripts,
            vec![
                TOGGLE_JS_PATH.to_owned(),
                APPS_JS_PATH.to_owned(),
                SHIKI_INIT_JS_PATH.to_owned(),
            ]
        );
    }

    #[test]
    fn frontmatter_extras_land_in_assets_deduped_in_section_order() {
        let sec1 =
            "+++\ntitle = \"S1\"\nstyles = [\"/extra.css\"]\nscripts = [\"/a.js\"]\n+++\nbody\n"
                .to_owned();
        let sec2 =
            "+++\ntitle = \"S2\"\nstyles = [\"/extra.css\"]\nscripts = [\"/b.js\"]\n+++\nbody\n"
                .to_owned();
        let files = make_files(&[("01.md", sec1), ("02.md", sec2)]);
        let input = CourseInput {
            slug: "course-c",
            manifest: &make_manifest_one_session("Course C", &["01.md", "02.md"]),
            files: &files,
        };
        let loaded = parse_course(&input).unwrap();
        // /extra.css appears only once, after the base stylesheet
        assert_eq!(
            loaded.session_assets[0]
                .styles
                .iter()
                .filter(|s| s.as_str() == "/extra.css")
                .count(),
            1
        );
        // scripts in section order, deduplicated
        assert!(
            loaded.session_assets[0]
                .scripts
                .contains(&"/a.js".to_owned())
        );
        assert!(
            loaded.session_assets[0]
                .scripts
                .contains(&"/b.js".to_owned())
        );
        let pos_a = loaded.session_assets[0]
            .scripts
            .iter()
            .position(|s| s == "/a.js")
            .unwrap();
        let pos_b = loaded.session_assets[0]
            .scripts
            .iter()
            .position(|s| s == "/b.js")
            .unwrap();
        assert!(pos_a < pos_b);
    }

    #[test]
    fn missing_section_file_returns_missing_section_error() {
        let files = make_files(&[("01-a.md", make_section("A", "body\n"))]);
        let input = CourseInput {
            slug: "course-d",
            manifest: &make_manifest_one_session("Course D", &["01-a.md", "missing.md"]),
            files: &files,
        };
        assert!(matches!(
            parse_course(&input),
            Err(Error::MissingSection(ref f)) if f == "missing.md"
        ));
    }

    #[test]
    fn broken_frontmatter_returns_invalid_section_naming_the_file() {
        let files = make_files(&[("01-bad.md", "no frontmatter fence\n".to_owned())]);
        let input = CourseInput {
            slug: "course-e",
            manifest: &make_manifest_one_session("Course E", &["01-bad.md"]),
            files: &files,
        };
        assert!(matches!(
            parse_course(&input),
            Err(Error::InvalidSection { ref file, .. }) if file == "01-bad.md"
        ));
    }

    #[test]
    fn invalid_slug_returns_invalid_slug_error() {
        let files = make_files(&[("01.md", make_section("T", "body\n"))]);
        let input = CourseInput {
            slug: "Not A Slug",
            manifest: &make_manifest_one_session("T", &["01.md"]),
            files: &files,
        };
        assert!(matches!(parse_course(&input), Err(Error::InvalidSlug(_))));
    }

    // --- new tests ---

    #[test]
    fn two_session_manifest_produces_two_sessions_in_order() {
        let files = make_files(&[
            ("01-a.md", make_section("Sec A", "body a\n")),
            ("02-b.md", make_section("Sec B", "body b\n")),
            ("03-c.md", make_section("Sec C", "body c\n")),
        ]);
        let manifest = concat!(
            "title = \"Two Sessions\"\n\n",
            "[[session]]\nslug = \"session-one\"\ntitle = \"Session One\"\nsections = [\"01-a.md\", \"02-b.md\"]\n\n",
            "[[session]]\nslug = \"session-two\"\ntitle = \"Session Two\"\nsections = [\"03-c.md\"]\n"
        );
        let input = CourseInput {
            slug: "two-session-course",
            manifest,
            files: &files,
        };
        let loaded = parse_course(&input).unwrap();
        assert_eq!(loaded.course.sessions.len(), 2);
        assert_eq!(loaded.session_assets.len(), 2);
        assert_eq!(loaded.course.sessions[0].slug.as_str(), "session-one");
        assert_eq!(loaded.course.sessions[0].title, "Session One");
        assert_eq!(loaded.course.sessions[0].sections.len(), 2);
        assert_eq!(loaded.course.sessions[1].slug.as_str(), "session-two");
        assert_eq!(loaded.course.sessions[1].title, "Session Two");
        assert_eq!(loaded.course.sessions[1].sections.len(), 1);
    }

    #[test]
    fn solution_in_session_two_only_injects_toggle_in_session_two_assets() {
        let sol_body = "Before\n::: solucion\nAnswer\n:::\nAfter\n";
        let files = make_files(&[
            ("01-plain.md", make_section("Plain", "No solutions.\n")),
            ("02-sol.md", make_section("With Sol", sol_body)),
        ]);
        let manifest = concat!(
            "title = \"Course\"\n\n",
            "[[session]]\nslug = \"s1\"\ntitle = \"Session 1\"\nsections = [\"01-plain.md\"]\n\n",
            "[[session]]\nslug = \"s2\"\ntitle = \"Session 2\"\nsections = [\"02-sol.md\"]\n"
        );
        let input = CourseInput {
            slug: "course-sol",
            manifest,
            files: &files,
        };
        let loaded = parse_course(&input).unwrap();
        assert!(
            !loaded.session_assets[0]
                .scripts
                .contains(&TOGGLE_JS_PATH.to_owned())
        );
        assert!(
            loaded.session_assets[1]
                .scripts
                .contains(&TOGGLE_JS_PATH.to_owned())
        );
    }

    #[test]
    fn invalid_session_slug_returns_invalid_slug_error() {
        let files = make_files(&[("01.md", make_section("T", "body\n"))]);
        let manifest = concat!(
            "title = \"Course\"\n\n",
            "[[session]]\nslug = \"Bad Slug\"\ntitle = \"Session\"\nsections = [\"01.md\"]\n"
        );
        let input = CourseInput {
            slug: "course-f",
            manifest,
            files: &files,
        };
        assert!(matches!(parse_course(&input), Err(Error::InvalidSlug(_))));
    }

    #[test]
    fn slide_blocks_collected_across_sections_in_order() {
        let sec1 = make_section("S1", ":::slide\nSlide A.\n:::\n");
        let sec2 = make_section("S2", ":::slide\nSlide B.\n:::\n:::slide\nSlide C.\n:::\n");
        let files = make_files(&[("01.md", sec1), ("02.md", sec2)]);
        let input = CourseInput {
            slug: "course-slides",
            manifest: &make_manifest_one_session("Slides Course", &["01.md", "02.md"]),
            files: &files,
        };
        let loaded = parse_course(&input).unwrap();
        let slides = &loaded.session_slides[0];
        assert_eq!(slides.len(), 3);
        assert!(slides[0].html.contains("Slide A"));
        assert!(slides[1].html.contains("Slide B"));
        assert!(slides[2].html.contains("Slide C"));
    }

    #[test]
    fn section_with_app_injects_apps_script() {
        let body = ":::app\n<cb-cpu-burst></cb-cpu-burst>\n:::\n";
        let files = make_files(&[("01-app.md", make_section("With App", body))]);
        let input = CourseInput {
            slug: "course-app",
            manifest: &make_manifest_one_session("Course App", &["01-app.md"]),
            files: &files,
        };
        let loaded = parse_course(&input).unwrap();
        assert!(
            loaded.session_assets[0]
                .scripts
                .contains(&APPS_JS_PATH.to_owned())
        );
    }

    #[test]
    fn section_without_app_does_not_inject_apps_script() {
        let files = make_files(&[("01-plain.md", make_section("Plain", "No apps.\n"))]);
        let input = CourseInput {
            slug: "course-no-app",
            manifest: &make_manifest_one_session("Course No App", &["01-plain.md"]),
            files: &files,
        };
        let loaded = parse_course(&input).unwrap();
        assert!(
            !loaded.session_assets[0]
                .scripts
                .contains(&APPS_JS_PATH.to_owned())
        );
    }

    #[test]
    fn section_with_a_language_fence_injects_shiki() {
        let body = "```yaml\nversion: 0.2\n```\n";
        let files = make_files(&[("01-code.md", make_section("Code", body))]);
        let input = CourseInput {
            slug: "course-code",
            manifest: &make_manifest_one_session("Course Code", &["01-code.md"]),
            files: &files,
        };

        let loaded = parse_course(&input).unwrap();
        assert!(
            loaded.session_assets[0]
                .scripts
                .contains(&SHIKI_INIT_JS_PATH.to_owned())
        );
    }

    #[test]
    fn section_with_an_http_app_injects_shiki() {
        // <cb-http> highlights its response panel through the on-demand
        // helper shiki-init.js exposes, so the app alone must pull Shiki in.
        let body = ":::app\n<cb-http endpoint=\"/eco/prueba\"></cb-http>\n:::\n";
        let files = make_files(&[("01-http.md", make_section("Http", body))]);
        let input = CourseInput {
            slug: "course-http",
            manifest: &make_manifest_one_session("Course Http", &["01-http.md"]),
            files: &files,
        };

        let loaded = parse_course(&input).unwrap();
        assert!(
            loaded.session_assets[0]
                .scripts
                .contains(&SHIKI_INIT_JS_PATH.to_owned())
        );
    }

    #[test]
    fn goto_app_resolves_across_sections_in_guide_and_slide_copies() {
        let sec1 = make_section(
            "S1",
            ":::inline-slide\n## Destino final\n:::app\n\
             <cb-goto path=\"Práctica guiada\"></cb-goto>\n:::\n:::\n",
        );
        let sec2 = make_section("S2", "## Práctica guiada\n\nPasos.\n");
        let files = make_files(&[("01.md", sec1), ("02.md", sec2)]);
        let input = CourseInput {
            slug: "course-goto",
            manifest: &make_manifest_one_session("Goto", &["01.md", "02.md"]),
            files: &files,
        };
        let loaded = parse_course(&input).unwrap();
        let guide = &loaded.course.sessions[0].sections[0].body_html;
        assert!(guide.contains("data-target=\"practica-guiada\""));
        let slide = &loaded.session_slides[0][0].html;
        assert!(slide.contains("data-target=\"practica-guiada\""));
    }

    #[test]
    fn goto_app_with_unknown_target_names_the_file() {
        let body = ":::app\n<cb-goto path=\"No existe\"></cb-goto>\n:::\n";
        let files = make_files(&[("01-goto.md", make_section("S1", body))]);
        let input = CourseInput {
            slug: "course-goto-bad",
            manifest: &make_manifest_one_session("Goto Bad", &["01-goto.md"]),
            files: &files,
        };
        let err = parse_course(&input).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidSection { ref file, ref message }
                if file == "01-goto.md" && message.contains("No existe")
        ));
    }

    #[test]
    fn session_with_no_slides_has_empty_slide_vec() {
        let files = make_files(&[("01.md", make_section("S1", "No slides here.\n"))]);
        let input = CourseInput {
            slug: "no-slides",
            manifest: &make_manifest_one_session("No Slides", &["01.md"]),
            files: &files,
        };
        let loaded = parse_course(&input).unwrap();
        assert!(loaded.session_slides[0].is_empty());
    }
}
