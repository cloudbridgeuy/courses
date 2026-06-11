use crate::course::Course;

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

/// Renders a course guide as a complete HTML document.
///
/// Titles are escaped; section bodies are trusted, authored HTML, and are
/// rendered verbatim.
pub fn render_guide_page(course: &Course) -> String {
    let mut sections = String::new();
    for section in &course.sections {
        sections.push_str("<section>\n<h2>");
        sections.push_str(&escape_html(&section.title));
        sections.push_str("</h2>\n");
        sections.push_str(&section.body_html);
        sections.push_str("\n</section>\n");
    }

    let title = escape_html(&course.title);
    format!(
        "<!doctype html>\n<html lang=\"es\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title}</title>\n</head>\n<body>\n<main>\n<h1>{title}</h1>\n\
         {sections}</main>\n</body>\n</html>\n"
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::course::{Course, CourseSlug, GuideSection};

    fn sample() -> Course {
        Course {
            slug: CourseSlug::parse("aws-devops").unwrap(),
            title: "Taller <AWS> & DevOps".to_owned(),
            sections: vec![GuideSection {
                title: "Semana 1".to_owned(),
                body_html: "<p>Hola</p>".to_owned(),
            }],
        }
    }

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
    fn render_escapes_titles() {
        let html = render_guide_page(&sample());
        assert!(html.contains("Taller &lt;AWS&gt; &amp; DevOps"));
        assert!(!html.contains("<AWS>"));
    }

    #[test]
    fn render_passes_body_html_verbatim() {
        let html = render_guide_page(&sample());
        assert!(html.contains("<p>Hola</p>"));
    }

    #[test]
    fn render_produces_a_document() {
        let html = render_guide_page(&sample());
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("<h2>Semana 1</h2>"));
    }

    #[test]
    fn render_escapes_title_in_both_places() {
        let html = render_guide_page(&sample());
        assert_eq!(html.matches("Taller &lt;AWS&gt; &amp; DevOps").count(), 2);
    }

    #[test]
    fn render_handles_empty_sections() {
        let course = Course {
            slug: CourseSlug::parse("aws-devops").unwrap(),
            title: "Vacío".to_owned(),
            sections: vec![],
        };
        let html = render_guide_page(&course);
        assert!(html.starts_with("<!doctype html>"));
        assert!(!html.contains("<section>"));
    }

    #[test]
    fn escape_html_handles_empty() {
        assert_eq!(escape_html(""), "");
    }
}
