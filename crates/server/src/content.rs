//! Embedded course content, parsed once at startup.
//!
//! The embed happens at compile time; everything below is pure computation
//! over those constants — no runtime I/O occurs in this module.

use color_eyre::eyre::{OptionExt, Result, WrapErr};
use courses_core::{CourseInput, LoadedCourse, RenderedSite};
use include_dir::{Dir, include_dir};

static CONTENT: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../content");

mod embedded_files {
    include!(concat!(env!("OUT_DIR"), "/embedded_file_apps.rs"));
}

/// Repository files referenced by `<cb-file>` elements at compile time.
pub fn embedded_file_paths() -> &'static [&'static str] {
    embedded_files::EMBEDDED_FILE_PATHS
}

/// Parses every course under `content/`, and pre-renders the full site.
pub fn load_site() -> Result<RenderedSite> {
    let mut courses: Vec<LoadedCourse> = Vec::new();
    for dir in CONTENT.dirs() {
        let slug = dir
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_eyre("content subdirectory with a non-UTF-8 name")?;
        courses.push(load_course(slug, dir).wrap_err_with(|| format!("course {slug:?}"))?);
    }
    courses.sort_by(|a, b| a.course.slug.as_str().cmp(b.course.slug.as_str()));
    Ok(courses_core::render_site(&courses))
}

fn load_course(slug: &str, dir: &Dir<'static>) -> Result<LoadedCourse> {
    let manifest = dir
        .get_file(dir.path().join("course.toml"))
        .and_then(|f| f.contents_utf8())
        .ok_or_eyre("missing, or non-UTF-8, course.toml")?;

    let mut files: Vec<(String, String)> = Vec::new();
    // dir.files() is shallow: a .md file nested in a subdirectory is not
    // collected, and a manifest referencing it fails loudly as MissingSection.
    for file in dir.files() {
        let Some(name) = file.path().file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.ends_with(".md") {
            continue;
        }
        let contents = file
            .contents_utf8()
            .ok_or_eyre(format!("non-UTF-8 section file: {name}"))?;
        let contents = crate::file_apps::expand_file_apps(contents, |path| {
            embedded_files::embedded_file(path).map(str::to_owned)
        })
        .wrap_err_with(|| format!("could not expand cb-file references in {name}"))?;
        files.push((name.to_owned(), contents));
    }

    let input = CourseInput {
        slug,
        manifest,
        files: &files,
    };
    courses_core::parse_course(&input).wrap_err("invalid course content")
}

// Exception to the no-shell-tests rule, by design (see the MVP 1 design doc):
// this test runs only pure core functions over compile-time constants. It
// guards the real authored content — broken content fails `cargo xtask lint`
// before the server ever boots.
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn embedded_content_parses_and_renders() {
        let site = load_site().unwrap();
        // landing page under the bare slug
        assert!(site.pages.contains_key("aws-devops"));
        // session pages under slug/session
        let session = &site.pages["aws-devops/del-codigo-a-la-imagen"];
        assert!(session.contains("class=\"solucion\""));
        assert!(session.contains("/static/toggle.js"));
        assert!(session.contains("<cb-file path=\"./buildspec.yml\" type=\"yaml\""));
        assert!(session.contains("data-content=\"version: 0.2"));
        let apps_at = session.find("/static/apps.js").unwrap();
        let shiki_at = session.find("/static/shiki-init.js").unwrap();
        assert!(apps_at < shiki_at, "cb-file must render before Shiki runs");
        // index tree links the session
        assert!(
            site.index_html
                .contains("/courses/aws-devops/del-codigo-a-la-imagen")
        );
    }
}
