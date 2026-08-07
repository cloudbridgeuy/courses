//! Pure helpers backing the development hot-reload mode.
//!
//! Nothing here performs I/O: these functions map names, filter paths, and
//! transform already-rendered HTML. The imperative shell in `courses_server`
//! supplies the file system, the watcher, and the HTTP surface.

use crate::render::RenderedSite;

/// A text asset the dev server may re-read from disk instead of serving the
/// copy embedded at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextAsset {
    /// The `Content-Type` header value the shell must send.
    pub content_type: &'static str,
    /// Path of the source file, relative to the repository root.
    pub repo_path: &'static str,
}

const JS: &str = "application/javascript; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";

/// The tag injected into every page while dev mode is on. The URL it points at
/// is the `dev-reload.js` arm of the server's `/static/{file}` handler.
pub const DEV_RELOAD_SCRIPT_TAG: &str = "<script src=\"/static/dev-reload.js\"></script>\n";

/// Maps a `/static/{file}` name to its hot-reloadable source file.
///
/// Returns `None` for binary assets, for vendored bundles that are never
/// hand-edited, and for unknown names. Acting as a whitelist, it is also what
/// keeps the shell from reading an arbitrary path off the request.
pub fn text_asset(file: &str) -> Option<TextAsset> {
    let asset = |content_type, repo_path| {
        Some(TextAsset {
            content_type,
            repo_path,
        })
    };
    match file {
        "guide.css" => asset(CSS, "crates/server/static/guide.css"),
        "slides.css" => asset(CSS, "crates/server/static/slides.css"),
        "cb-widgets.css" => asset(CSS, "crates/server/static/cb-widgets.css"),
        "apps.js" => asset(JS, "crates/server/static/apps.js"),
        "toggle.js" => asset(JS, "crates/server/static/toggle.js"),
        "shiki-init.js" => asset(JS, "crates/server/static/shiki-init.js"),
        "mermaid-init.js" => asset(JS, "crates/server/static/mermaid-init.js"),
        "vars.js" => asset(JS, "crates/server/static/vars.js"),
        _ => None,
    }
}

/// Decides whether a changed path should trigger a reload.
///
/// Editors write throwaway files beside the real one — swap files, backup
/// copies, and atomic-save probes — so an unfiltered watcher fires several
/// times per save. Only the four hand-edited extensions qualify, and hidden
/// files never do.
pub fn is_reload_trigger(path: &str) -> bool {
    // `rsplit` always yields at least one item, so the default is unreachable;
    // it exists only to keep the function total.
    let name = path.rsplit('/').next().unwrap_or_default();
    if name.starts_with('.') {
        return false;
    }
    matches!(
        name.rsplit_once('.'),
        Some((_, "md" | "toml" | "css" | "js"))
    )
}

/// Inserts `snippet` immediately before the first `</head>` of `html`.
///
/// Returns `None` when the document has no `</head>`, so the caller can report
/// a failed injection rather than silently serve a page with no reload client.
fn inject_before_head_close(html: &str, snippet: &str) -> Option<String> {
    let at = html.find("</head>")?;
    let mut out = String::with_capacity(html.len() + snippet.len());
    out.push_str(&html[..at]);
    out.push_str(snippet);
    out.push_str(&html[at..]);
    Some(out)
}

/// Applies [`inject_before_head_close`] to every page of a rendered site.
///
/// Returns the transformed site, and the keys of the pages that carried no
/// `</head>`, which are left untouched. The shell logs those keys; the site
/// still serves, so one malformed template never blocks a reload.
pub fn with_dev_script(site: RenderedSite, snippet: &str) -> (RenderedSite, Vec<String>) {
    let mut skipped = Vec::new();
    let index_html = inject_or_skip(site.index_html, snippet, "index", &mut skipped);
    let not_found_html = inject_or_skip(site.not_found_html, snippet, "not-found", &mut skipped);
    let pages = site
        .pages
        .into_iter()
        .map(|(key, html)| {
            let html = inject_or_skip(html, snippet, &key, &mut skipped);
            (key, html)
        })
        .collect();

    (
        RenderedSite {
            index_html,
            pages,
            not_found_html,
        },
        skipped,
    )
}

/// Injects into `html`, recording `key` when the document has no `</head>`.
fn inject_or_skip(html: String, snippet: &str, key: &str, skipped: &mut Vec<String>) -> String {
    match inject_before_head_close(&html, snippet) {
        Some(injected) => injected,
        None => {
            skipped.push(key.to_owned());
            html
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn every_hot_asset_maps_to_its_source_file() {
        let cases = [
            ("guide.css", CSS, "crates/server/static/guide.css"),
            ("slides.css", CSS, "crates/server/static/slides.css"),
            ("cb-widgets.css", CSS, "crates/server/static/cb-widgets.css"),
            ("apps.js", JS, "crates/server/static/apps.js"),
            ("toggle.js", JS, "crates/server/static/toggle.js"),
            ("shiki-init.js", JS, "crates/server/static/shiki-init.js"),
            (
                "mermaid-init.js",
                JS,
                "crates/server/static/mermaid-init.js",
            ),
            ("vars.js", JS, "crates/server/static/vars.js"),
        ];
        for (file, content_type, repo_path) in cases {
            let found = text_asset(file).unwrap();
            assert_eq!(found.content_type, content_type, "content type for {file}");
            assert_eq!(found.repo_path, repo_path, "repo path for {file}");
        }
    }

    #[test]
    fn binary_assets_are_not_hot_reloadable() {
        for file in [
            "favicon.ico",
            "favicon.png",
            "montserrat.ttf",
            "cloudbridge.png",
        ] {
            assert!(text_asset(file).is_none(), "{file} must stay embedded");
        }
    }

    #[test]
    fn vendored_bundles_are_not_hot_reloadable() {
        for file in ["reveal.min.js", "reveal.min.css", "mermaid.min.js"] {
            assert!(text_asset(file).is_none(), "{file} must stay embedded");
        }
    }

    #[test]
    fn unknown_and_traversing_names_are_rejected() {
        for file in [
            "",
            "nope.css",
            "../Cargo.toml",
            "guide.css.bak",
            "GUIDE.CSS",
        ] {
            assert!(text_asset(file).is_none(), "{file:?} must not resolve");
        }
    }

    #[test]
    fn content_and_asset_edits_trigger_a_reload() {
        for path in [
            "content/aws-devops/01-introduccion.md",
            "content/aws-devops/course.toml",
            "crates/server/static/guide.css",
            "crates/server/static/apps.js",
            "01.md",
        ] {
            assert!(is_reload_trigger(path), "{path} must trigger");
        }
    }

    #[test]
    fn editor_noise_does_not_trigger_a_reload() {
        for path in [
            "content/aws-devops/.DS_Store",
            "content/aws-devops/.01-introduccion.md.swp",
            "content/aws-devops/01-introduccion.md~",
            "content/aws-devops/01-introduccion.md.swp",
            "content/aws-devops/guide.css___jb_tmp___",
            "content/aws-devops/4913",
            "content/aws-devops",
            "",
        ] {
            assert!(!is_reload_trigger(path), "{path:?} must not trigger");
        }
    }

    use std::collections::HashMap;

    const SNIPPET: &str = "<script src=\"/x.js\"></script>";

    fn doc(body: &str) -> String {
        format!(
            "<!doctype html>\n<html>\n<head>\n<title>t</title></head>\n<body>\n{body}</body>\n</html>\n"
        )
    }

    #[test]
    fn injects_immediately_before_the_head_close() {
        let html = inject_before_head_close(&doc("x"), SNIPPET).unwrap();
        let expected = format!("{SNIPPET}</head>");
        assert!(
            html.contains(&expected),
            "snippet must abut </head>: {html}"
        );
        assert_eq!(html.matches(SNIPPET).count(), 1);
    }

    #[test]
    fn missing_head_close_returns_none() {
        assert!(inject_before_head_close("<html><body>x</body></html>", SNIPPET).is_none());
    }

    #[test]
    fn only_the_first_head_close_receives_the_snippet() {
        let html = inject_before_head_close("<head></head><head></head>", SNIPPET).unwrap();
        assert_eq!(html, format!("<head>{SNIPPET}</head><head></head>"));
    }

    #[test]
    fn with_dev_script_covers_index_pages_and_not_found() {
        let mut pages = HashMap::new();
        pages.insert("a".to_owned(), doc("a"));
        pages.insert("a/b".to_owned(), doc("b"));
        let site = RenderedSite {
            index_html: doc("index"),
            pages,
            not_found_html: doc("404"),
        };

        let (site, skipped) = with_dev_script(site, SNIPPET);

        assert!(skipped.is_empty(), "nothing should be skipped: {skipped:?}");
        assert!(site.index_html.contains(SNIPPET));
        assert!(site.not_found_html.contains(SNIPPET));
        assert!(site.pages["a"].contains(SNIPPET));
        assert!(site.pages["a/b"].contains(SNIPPET));
    }

    #[test]
    fn a_page_without_head_close_is_reported_and_left_untouched() {
        let mut pages = HashMap::new();
        pages.insert("broken".to_owned(), "<html>no head</html>".to_owned());
        let site = RenderedSite {
            index_html: doc("index"),
            pages,
            not_found_html: doc("404"),
        };

        let (site, skipped) = with_dev_script(site, SNIPPET);

        assert_eq!(skipped, vec!["broken".to_owned()]);
        assert_eq!(site.pages["broken"], "<html>no head</html>");
        assert!(site.index_html.contains(SNIPPET));
    }
}
