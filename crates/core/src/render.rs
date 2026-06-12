use std::collections::HashMap;
use std::fmt::Write as FmtWrite;

use crate::assets::{PageAssets, REVEAL_CSS_PATH, REVEAL_JS_PATH, SLIDES_CSS_PATH};
use crate::catalog::LoadedCourse;
use crate::course::{Course, Session};
use crate::solutions::SlideFragment;

/// Escapes the five HTML-significant characters in `raw`.
pub fn escape_html(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

fn render_head_assets(assets: &PageAssets) -> String {
    let mut out = String::new();
    for href in &assets.styles {
        // Infallible: writing to a String never fails.
        let _ = writeln!(
            out,
            "<link rel=\"stylesheet\" href=\"{}\">",
            escape_html(href)
        );
    }
    for src in &assets.scripts {
        let _ = writeln!(out, "<script defer src=\"{}\"></script>", escape_html(src));
    }
    out
}

/// URL of the embedded brand logo (white wordmark, colored cloud, transparent).
const LOGO_PATH: &str = "/static/cloudbridge.png";

/// URL of the all-white brand logo, for the dark footer.
const LOGO_WHITE_PATH: &str = "/static/cloudbridge-white.png";

/// The dark header band: the brand logo, linking home, over the gradient strip.
fn header_html() -> String {
    format!(
        "<header class=\"cb-header\">\n\
         <a href=\"/\" aria-label=\"CloudBridge\">\
         <img src=\"{LOGO_PATH}\" alt=\"CloudBridge\" width=\"180\" height=\"40\">\
         </a>\n</header>\n<div class=\"cb-strip\"></div>\n"
    )
}

/// The dark footer band: a small logo, and the company line.
fn footer_html() -> String {
    format!(
        "<footer class=\"cb-footer\">\n\
         <img src=\"{LOGO_WHITE_PATH}\" alt=\"CloudBridge\" width=\"110\" height=\"24\">\
         <span>© Cloud Bridge SAS</span>\n</footer>\n"
    )
}

fn page(lang: &str, title_html: &str, head_extra: &str, main_html: &str) -> String {
    let header = header_html();
    let footer = footer_html();
    format!(
        "<!doctype html>\n<html id=\"top\" lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title_html}</title>\n{head_extra}</head>\n<body>\n\
         {header}<main>\n{main_html}</main>\n{footer}\
         <a href=\"#top\" id=\"cb-top-btn\" aria-label=\"Volver arriba\">↑</a>\n\
         <script>var b=document.getElementById('cb-top-btn'),f=document.querySelector('.cb-footer');\
window.addEventListener('scroll',function(){{b.classList.toggle('cb-top-visible',scrollY>300);}});\
new IntersectionObserver(function(e){{b.style.bottom=e[0].isIntersecting?(f.offsetHeight+12)+'px':'';}}).observe(f);\
</script>\n\
         </body>\n</html>\n"
    )
}

/// Renders a course→session→section tree as nested lists.
///
/// Session titles link to `/courses/{course}/{session}`; section titles link
/// to that session page's `seccion-N` anchor.
pub fn render_tree(courses: &[LoadedCourse]) -> String {
    let mut out = String::from("<ul class=\"cb-tree\">\n");
    for loaded in courses {
        let course_slug = escape_html(loaded.course.slug.as_str());
        let course_title = escape_html(&loaded.course.title);
        out.push_str(&format!(
            "<li><details class=\"cb-tree-course\" open>\
             <summary>{course_title}</summary>\n<ul>\n"
        ));
        for session in &loaded.course.sessions {
            let session_slug = escape_html(session.slug.as_str());
            let session_title = escape_html(&session.title);
            out.push_str(&format!(
                "<li class=\"cb-tree-session\">\
                 <a href=\"/courses/{course_slug}/{session_slug}\">{session_title}</a>\n<ul>\n"
            ));
            for (index, section) in session.sections.iter().enumerate() {
                let n = index + 1;
                let section_title = escape_html(&section.title);
                out.push_str(&format!(
                    "<li><a href=\"/courses/{course_slug}/{session_slug}#seccion-{n}\">\
                     {section_title}</a></li>\n"
                ));
            }
            out.push_str("</ul>\n</li>\n");
        }
        out.push_str("</ul>\n</details></li>\n");
    }
    out.push_str("</ul>\n");
    out
}

/// Renders the course index (`GET /`): the full course→session→section tree.
pub fn render_index_page(courses: &[LoadedCourse]) -> String {
    let main = format!("<h1>Courses</h1>\n{}", render_tree(courses));
    page(
        "en",
        "Courses",
        &render_head_assets(&PageAssets::base()),
        &main,
    )
}

/// Renders a course landing page (`GET /courses/{slug}`): the title, and the
/// course's session→section tree.
pub fn render_landing_page(loaded: &LoadedCourse) -> String {
    let title = escape_html(&loaded.course.title);
    let tree = render_tree(std::slice::from_ref(loaded));
    let main = format!("<h1>{title}</h1>\n{tree}");
    page(
        "es",
        &title,
        &render_head_assets(&PageAssets::base()),
        &main,
    )
}

/// Inputs for rendering one session page.
pub struct SessionPage<'a> {
    pub course: &'a Course,
    pub session: &'a Session,
    pub assets: &'a PageAssets,
    pub prev: Option<&'a Session>,
    pub next: Option<&'a Session>,
    pub has_slides: bool,
}

/// Renders one session as a complete HTML document: the section nav, the
/// sections (each a `seccion-N` anchor), and the footer session nav.
pub fn render_session_page(input: &SessionPage<'_>) -> String {
    let course_slug = escape_html(input.course.slug.as_str());

    let mut nav_items = String::new();
    let mut sections = String::new();
    for (index, section) in input.session.sections.iter().enumerate() {
        let n = index + 1;
        let title = escape_html(&section.title);
        nav_items.push_str(&format!("<li><a href=\"#seccion-{n}\">{title}</a></li>\n"));
        sections.push_str(&format!(
            "<section id=\"seccion-{n}\">\n<h2>{title}</h2>\n{}\n</section>\n",
            section.body_html
        ));
    }
    let nav = if nav_items.is_empty() {
        String::new()
    } else {
        format!("<nav aria-label=\"Contenido\">\n<ol>\n{nav_items}</ol>\n</nav>\n")
    };

    let footer_nav = render_session_footer_nav(&course_slug, input.prev, input.next);

    let title = escape_html(&input.session.title);
    let slides_link = if input.has_slides {
        format!(
            "<p class=\"cb-slides-link\"><a href=\"/courses/{course_slug}/{session_slug}/slides\">▶ Ver diapositivas</a></p>\n",
            session_slug = escape_html(input.session.slug.as_str())
        )
    } else {
        String::new()
    };
    let main = format!("<h1>{title}</h1>\n{slides_link}{nav}{sections}{footer_nav}");
    page("es", &title, &render_head_assets(input.assets), &main)
}

/// Renders a reveal.js slideshow for one session.
///
/// Each element of `slides_html` becomes one `<section>` in the `.slides`
/// container. Returns an empty-slides page when `slides_html` is empty (the
/// route key is only inserted by `render_site` when slides are non-empty, so
/// this is a safety net for direct callers).
pub fn render_slideshow_page(
    course: &Course,
    session: &Session,
    slides_html: &[SlideFragment],
) -> String {
    let course_slug = escape_html(course.slug.as_str());
    let session_slug = escape_html(session.slug.as_str());
    let title = escape_html(&session.title);

    let sections: String = slides_html
        .iter()
        .map(|slide| {
            let class = if slide.light {
                " class=\"cb-light\""
            } else {
                ""
            };
            format!("<section{class}>\n{}</section>\n", slide.html)
        })
        .collect();

    format!(
        "<!doctype html>\n<html lang=\"es\">\n<head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title} — Diapositivas</title>\n\
         <link rel=\"stylesheet\" href=\"{REVEAL_CSS_PATH}\">\n\
         <link rel=\"stylesheet\" href=\"{SLIDES_CSS_PATH}\">\n\
         </head>\n<body>\n\
         <div class=\"reveal\">\n<div class=\"slides\">\n\
         {sections}\
         </div>\n</div>\n\
         <script src=\"{REVEAL_JS_PATH}\"></script>\n\
         <script>\
Reveal.initialize({{hash:true,controls:true,progress:true,center:true,transition:'slide'}});\
</script>\n\
         <a class=\"cb-slides-close\" href=\"/courses/{course_slug}/{session_slug}\" \
aria-label=\"Cerrar diapositivas\">✕</a>\n\
         </body>\n</html>\n"
    )
}

/// The bottom session nav: previous, next, and back-to-course links.
fn render_session_footer_nav(
    course_slug: &str,
    prev: Option<&Session>,
    next: Option<&Session>,
) -> String {
    let mut out = String::from("<nav class=\"cb-session-nav\" aria-label=\"Sesiones\">\n");
    if let Some(prev) = prev {
        let slug = escape_html(prev.slug.as_str());
        out.push_str(&format!(
            "<a class=\"cb-prev\" href=\"/courses/{course_slug}/{slug}\">← Sesión anterior</a>\n"
        ));
    }
    out.push_str(&format!(
        "<a class=\"cb-up\" href=\"/courses/{course_slug}\">↑ Volver al curso</a>\n"
    ));
    if let Some(next) = next {
        let slug = escape_html(next.slug.as_str());
        out.push_str(&format!(
            "<a class=\"cb-next\" href=\"/courses/{course_slug}/{slug}\">Sesión siguiente →</a>\n"
        ));
    }
    out.push_str("</nav>\n");
    out
}

/// Renders the 404 page (platform chrome — English).
pub fn render_not_found_page() -> String {
    let main = "<h1>404 — Not Found</h1>\n\
                <p>Nothing lives at this address. <a href=\"/\">Back to the course list</a>.</p>\n";
    page(
        "en",
        "404 — Not Found",
        &render_head_assets(&PageAssets::base()),
        main,
    )
}

/// Every page the site serves, pre-rendered: the index, one page per URL path
/// (landing or session), and the 404 body. Pages are keyed by URL path —
/// course slug (landing) or `slug/session` (session page).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedSite {
    pub index_html: String,
    pub pages: HashMap<String, String>,
    pub not_found_html: String,
}

/// Renders the whole site from parsed courses (CQRS: built once; handlers look
/// strings up by URL path).
pub fn render_site(courses: &[LoadedCourse]) -> RenderedSite {
    let mut pages = HashMap::new();
    for loaded in courses {
        let course_slug = loaded.course.slug.as_str();
        pages.insert(course_slug.to_owned(), render_landing_page(loaded));

        let sessions = &loaded.course.sessions;
        for (i, session) in sessions.iter().enumerate() {
            let slides = &loaded.session_slides[i];
            let input = SessionPage {
                course: &loaded.course,
                session,
                assets: &loaded.session_assets[i],
                prev: i.checked_sub(1).map(|p| &sessions[p]),
                next: sessions.get(i + 1),
                has_slides: !slides.is_empty(),
            };
            let key = format!("{course_slug}/{}", session.slug.as_str());
            pages.insert(key, render_session_page(&input));

            if !slides.is_empty() {
                let slides_key = format!("{course_slug}/{}/slides", session.slug.as_str());
                pages.insert(
                    slides_key,
                    render_slideshow_page(&loaded.course, session, slides),
                );
            }
        }
    }
    RenderedSite {
        index_html: render_index_page(courses),
        pages,
        not_found_html: render_not_found_page(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::assets::{GUIDE_CSS_PATH, PageAssets};
    use crate::catalog::LoadedCourse;
    use crate::course::{Course, CourseSlug, GuideSection, Session, SessionSlug};

    fn sample() -> LoadedCourse {
        LoadedCourse {
            course: Course {
                slug: CourseSlug::parse("aws-devops").unwrap(),
                title: "Taller <AWS> & DevOps".to_owned(),
                sessions: vec![Session {
                    slug: SessionSlug::parse("semana-1").unwrap(),
                    title: "Semana 1".to_owned(),
                    sections: vec![GuideSection {
                        title: "Sección 1".to_owned(),
                        body_html: "<p>Hola</p>".to_owned(),
                    }],
                }],
            },
            session_assets: vec![PageAssets::base()],
            session_slides: vec![vec![]],
        }
    }

    fn sample_session_page(loaded: &LoadedCourse) -> SessionPage<'_> {
        SessionPage {
            course: &loaded.course,
            session: &loaded.course.sessions[0],
            assets: &loaded.session_assets[0],
            prev: None,
            next: None,
            has_slides: false,
        }
    }

    // ── brand chrome ──────────────────────────────────────────────────────

    #[test]
    fn session_page_has_brand_chrome() {
        let loaded = sample();
        let html = render_session_page(&sample_session_page(&loaded));
        assert!(html.contains("<header class=\"cb-header\">"));
        assert!(html.contains("/static/cloudbridge.png"));
        assert!(html.contains("<div class=\"cb-strip\"></div>"));
        assert!(html.contains("<footer class=\"cb-footer\">"));
        assert!(html.contains("© Cloud Bridge SAS"));
        let header_at = html.find("cb-header").unwrap();
        let main_at = html.find("<main>").unwrap();
        let footer_at = html.find("cb-footer\">").unwrap();
        assert!(header_at < main_at);
        assert!(main_at < footer_at);
    }

    #[test]
    fn index_and_not_found_share_the_chrome() {
        for html in [render_index_page(&[]), render_not_found_page()] {
            assert!(html.contains("cb-header"));
            assert!(html.contains("cb-footer"));
            assert!(html.contains("/static/cloudbridge.png"));
        }
    }

    // ── escape_html ───────────────────────────────────────────────────────

    #[test]
    fn escape_html_escapes_all_five() {
        assert_eq!(
            escape_html(r#"<a href="x">&'"#),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;"
        );
    }

    #[test]
    fn escape_html_passes_plain_text() {
        assert_eq!(escape_html("hola"), "hola");
    }

    #[test]
    fn escape_html_handles_empty() {
        assert_eq!(escape_html(""), "");
    }

    // ── render_session_page ───────────────────────────────────────────────

    #[test]
    fn render_escapes_titles() {
        let loaded = sample();
        // Course title escaping: render_landing_page puts the course title in <h1> and <title>.
        let landing_html = render_landing_page(&loaded);
        assert!(landing_html.contains("Taller &lt;AWS&gt; &amp; DevOps"));
        assert!(!landing_html.contains("Taller <AWS>"));
        // Session title: render_session_page escapes the session title in <h1> and <title>.
        let mut loaded_unsafe = sample();
        loaded_unsafe.course.sessions[0].title = "Semana <1> & Intro".to_owned();
        let session_html = render_session_page(&sample_session_page(&loaded_unsafe));
        assert!(session_html.contains("Semana &lt;1&gt; &amp; Intro"));
        assert!(!session_html.contains("Semana <1>"));
    }

    #[test]
    fn render_passes_body_html_verbatim() {
        let loaded = sample();
        let html = render_session_page(&sample_session_page(&loaded));
        assert!(html.contains("<p>Hola</p>"));
    }

    #[test]
    fn render_produces_a_document() {
        let loaded = sample();
        let html = render_session_page(&sample_session_page(&loaded));
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("<h2>Sección 1</h2>") || html.contains("Secci"));
    }

    #[test]
    fn assets_emit_link_stylesheet() {
        let loaded = sample();
        let html = render_session_page(&sample_session_page(&loaded));
        assert!(html.contains(&format!(
            "<link rel=\"stylesheet\" href=\"{GUIDE_CSS_PATH}\">"
        )));
    }

    #[test]
    fn script_asset_emits_script_defer() {
        let mut assets = PageAssets::base();
        assets.push_script("/static/toggle.js");
        let loaded = sample();
        let input = SessionPage {
            course: &loaded.course,
            session: &loaded.course.sessions[0],
            assets: &assets,
            prev: None,
            next: None,
            has_slides: false,
        };
        let html = render_session_page(&input);
        assert!(html.contains("<script defer src=\"/static/toggle.js\">"));
    }

    #[test]
    fn render_session_page_has_seccion_id() {
        let loaded = sample();
        let html = render_session_page(&sample_session_page(&loaded));
        assert!(html.contains("id=\"seccion-1\""));
    }

    #[test]
    fn render_session_page_has_nav() {
        let loaded = sample();
        let html = render_session_page(&sample_session_page(&loaded));
        assert!(html.contains("<nav aria-label=\"Contenido\">"));
    }

    #[test]
    fn render_session_page_footer_nav_first_session_no_prev() {
        let loaded = sample();
        let next = Session {
            slug: SessionSlug::parse("semana-2").unwrap(),
            title: "Semana 2".to_owned(),
            sections: vec![],
        };
        let input = SessionPage {
            course: &loaded.course,
            session: &loaded.course.sessions[0],
            assets: &loaded.session_assets[0],
            prev: None,
            next: Some(&next),
            has_slides: false,
        };
        let html = render_session_page(&input);
        assert!(!html.contains("cb-prev"));
        assert!(html.contains("cb-up"));
        assert!(html.contains("cb-next"));
    }

    #[test]
    fn render_session_page_footer_nav_last_session_no_next() {
        let loaded = sample();
        let prev = Session {
            slug: SessionSlug::parse("semana-0").unwrap(),
            title: "Semana 0".to_owned(),
            sections: vec![],
        };
        let input = SessionPage {
            course: &loaded.course,
            session: &loaded.course.sessions[0],
            assets: &loaded.session_assets[0],
            prev: Some(&prev),
            next: None,
            has_slides: false,
        };
        let html = render_session_page(&input);
        assert!(!html.contains("cb-next"));
        assert!(html.contains("cb-up"));
        assert!(html.contains("cb-prev"));
    }

    #[test]
    fn render_session_page_footer_nav_middle_session_has_both() {
        let loaded = sample();
        let prev = Session {
            slug: SessionSlug::parse("semana-0").unwrap(),
            title: "Semana 0".to_owned(),
            sections: vec![],
        };
        let next = Session {
            slug: SessionSlug::parse("semana-2").unwrap(),
            title: "Semana 2".to_owned(),
            sections: vec![],
        };
        let input = SessionPage {
            course: &loaded.course,
            session: &loaded.course.sessions[0],
            assets: &loaded.session_assets[0],
            prev: Some(&prev),
            next: Some(&next),
            has_slides: false,
        };
        let html = render_session_page(&input);
        assert!(html.contains("cb-prev"));
        assert!(html.contains("cb-up"));
        assert!(html.contains("cb-next"));
    }

    // ── render_tree ───────────────────────────────────────────────────────

    #[test]
    fn render_tree_nests_course_session_section() {
        let loaded = sample();
        let html = render_tree(std::slice::from_ref(&loaded));
        assert!(html.contains("class=\"cb-tree\""));
        assert!(html.contains("/courses/aws-devops/semana-1"));
        assert!(html.contains("/courses/aws-devops/semana-1#seccion-1"));
    }

    // ── render_landing_page ───────────────────────────────────────────────

    #[test]
    fn render_landing_page_is_lang_es() {
        let loaded = sample();
        let html = render_landing_page(&loaded);
        assert!(html.contains("lang=\"es\""));
    }

    #[test]
    fn render_landing_page_contains_tree() {
        let loaded = sample();
        let html = render_landing_page(&loaded);
        assert!(html.contains("cb-tree"));
    }

    // ── render_index_page ─────────────────────────────────────────────────

    fn make_loaded(slug: &str, title: &str) -> LoadedCourse {
        LoadedCourse {
            course: Course {
                slug: CourseSlug::parse(slug).unwrap(),
                title: title.to_owned(),
                sessions: vec![Session {
                    slug: SessionSlug::parse("s1").unwrap(),
                    title: "Session 1".to_owned(),
                    sections: vec![],
                }],
            },
            session_assets: vec![PageAssets::base()],
            session_slides: vec![vec![]],
        }
    }

    #[test]
    fn render_index_page_contains_tree() {
        let courses = vec![make_loaded("aws-devops", "AWS DevOps")];
        let html = render_index_page(&courses);
        assert!(html.contains("cb-tree"));
        assert!(html.contains("/courses/aws-devops/s1"));
    }

    #[test]
    fn index_page_shows_each_course_title() {
        let courses = vec![
            make_loaded("aws-devops", "AWS DevOps"),
            make_loaded("kubernetes", "Kubernetes"),
        ];
        let html = render_index_page(&courses);
        assert!(html.contains("<summary>AWS DevOps</summary>"));
        assert!(html.contains("<summary>Kubernetes</summary>"));
    }

    #[test]
    fn index_page_escapes_special_chars_in_title() {
        let courses = vec![make_loaded("x", "A & B <test>")];
        let html = render_index_page(&courses);
        assert!(html.contains("A &amp; B &lt;test&gt;"));
    }

    // ── render_not_found_page ─────────────────────────────────────────────

    #[test]
    fn not_found_page_is_lang_en() {
        let html = render_not_found_page();
        assert!(html.contains("lang=\"en\""));
    }

    #[test]
    fn not_found_page_links_to_root() {
        let html = render_not_found_page();
        assert!(html.contains("<a href=\"/\">"));
    }

    // ── slides link ──────────────────────────────────────────────────────

    #[test]
    fn session_page_with_slides_shows_slides_link() {
        let loaded = sample();
        let input = SessionPage {
            course: &loaded.course,
            session: &loaded.course.sessions[0],
            assets: &loaded.session_assets[0],
            prev: None,
            next: None,
            has_slides: true,
        };
        let html = render_session_page(&input);
        assert!(html.contains("Ver diapositivas"));
        assert!(html.contains("/courses/aws-devops/semana-1/slides"));
    }

    #[test]
    fn session_page_without_slides_hides_slides_link() {
        let loaded = sample();
        let html = render_session_page(&sample_session_page(&loaded));
        assert!(!html.contains("Ver diapositivas"));
    }

    #[test]
    fn slideshow_page_wraps_each_slide_in_section_tag() {
        let loaded = sample();
        let slides = vec![
            SlideFragment {
                html: "<p>Slide one</p>\n".to_owned(),
                light: false,
            },
            SlideFragment {
                html: "<p>Slide two</p>\n".to_owned(),
                light: false,
            },
        ];
        let html = render_slideshow_page(&loaded.course, &loaded.course.sessions[0], &slides);
        assert!(html.contains("<div class=\"reveal\">"));
        assert!(html.contains("<div class=\"slides\">"));
        let count = html.matches("<section>").count();
        assert_eq!(count, 2);
        assert!(html.contains("Slide one"));
        assert!(html.contains("Slide two"));
        assert!(html.contains("reveal.min.js"));
        assert!(html.contains("Diapositivas"));
        assert!(html.contains("cb-slides-close"));
        assert!(html.contains("aria-label=\"Cerrar diapositivas\""));
        assert!(html.contains("slides.css"));
        assert!(!html.contains("reveal-theme-black"));
        // close button must appear before </body>
        assert!(html.find("cb-slides-close").unwrap() < html.find("</body>").unwrap());
    }

    #[test]
    fn slideshow_close_button_links_to_session_guide() {
        let loaded = sample();
        let slides = vec![SlideFragment {
            html: "<p>One</p>\n".to_owned(),
            light: false,
        }];
        let html = render_slideshow_page(&loaded.course, &loaded.course.sessions[0], &slides);
        assert!(
            html.contains("<a class=\"cb-slides-close\" href=\"/courses/aws-devops/semana-1\"")
        );
    }

    #[test]
    fn light_slide_emits_cb_light_class_dark_slide_does_not() {
        let loaded = sample();
        let slides = vec![
            SlideFragment {
                html: "<p>Dark</p>\n".to_owned(),
                light: false,
            },
            SlideFragment {
                html: "<p>Light</p>\n".to_owned(),
                light: true,
            },
        ];
        let html = render_slideshow_page(&loaded.course, &loaded.course.sessions[0], &slides);
        assert!(html.contains("<section>\n<p>Dark</p>"));
        assert!(html.contains("<section class=\"cb-light\">\n<p>Light</p>"));
    }

    // ── render_site ───────────────────────────────────────────────────────

    #[test]
    fn render_site_keys_pages_by_slug() {
        let courses = vec![make_loaded("aws-devops", "AWS DevOps")];
        let site = render_site(&courses);
        assert!(site.pages.contains_key("aws-devops"));
        assert!(site.pages.contains_key("aws-devops/s1"));
    }

    #[test]
    fn render_site_fills_all_three_fields() {
        let courses = vec![make_loaded("my-course", "My Course")];
        let site = render_site(&courses);
        assert!(!site.index_html.is_empty());
        assert!(!site.pages.is_empty());
        assert!(!site.not_found_html.is_empty());
    }

    #[test]
    fn render_site_keys_slideshow_page_when_slides_present() {
        let slides_html = vec![SlideFragment {
            html: "<p>One</p>\n".to_owned(),
            light: false,
        }];
        let mut courses = vec![make_loaded("aws-devops", "AWS DevOps")];
        courses[0].session_slides = vec![slides_html];
        let site = render_site(&courses);
        assert!(site.pages.contains_key("aws-devops/s1/slides"));
    }

    #[test]
    fn render_site_skips_slideshow_page_when_no_slides() {
        let courses = vec![make_loaded("aws-devops", "AWS DevOps")];
        let site = render_site(&courses);
        assert!(!site.pages.contains_key("aws-devops/s1/slides"));
    }
}
