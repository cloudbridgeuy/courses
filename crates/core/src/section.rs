use serde::Deserialize;

use crate::error::{Error, Result};

/// TOML frontmatter at the head of a section's Markdown file, between `+++` fences.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Frontmatter {
    pub title: String,
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default)]
    pub styles: Vec<String>,
}

/// Splits a section file into its raw frontmatter TOML, and its Markdown body.
///
/// The file must open with a `+++` line; the next `+++` line closes the
/// frontmatter, and everything after it is the body.
pub fn split_frontmatter(raw: &str) -> Result<(&str, &str)> {
    let rest = raw
        .strip_prefix("+++\n")
        .ok_or_else(|| Error::InvalidFrontmatter("missing opening +++".to_owned()))?;
    let Some(end) = rest.find("\n+++") else {
        return Err(Error::InvalidFrontmatter("missing closing +++".to_owned()));
    };
    let frontmatter = &rest[..end];
    let after_fence = &rest[end + "\n+++".len()..];
    // Reject "+++ trailing text" — closing fence must be alone on its line
    if !after_fence.is_empty() && !after_fence.starts_with('\n') {
        return Err(Error::InvalidFrontmatter(
            "closing +++ must be alone on its line".to_owned(),
        ));
    }
    let body = after_fence.strip_prefix('\n').unwrap_or(after_fence);
    Ok((frontmatter, body))
}

/// Parses raw frontmatter TOML into a [`Frontmatter`].
pub fn parse_frontmatter(raw: &str) -> Result<Frontmatter> {
    let frontmatter: Frontmatter =
        toml::from_str(raw).map_err(|e| Error::InvalidFrontmatter(e.to_string()))?;
    if frontmatter.title.trim().is_empty() {
        return Err(Error::InvalidFrontmatter(
            "title must not be empty".to_owned(),
        ));
    }
    Ok(frontmatter)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    const SAMPLE: &str = "+++\ntitle = \"Intro\"\n+++\n## Hello\n\nWorld\n";

    #[test]
    fn split_happy_path() {
        let (fm, body) = split_frontmatter(SAMPLE).unwrap();
        assert_eq!(fm, "title = \"Intro\"");
        assert_eq!(body, "## Hello\n\nWorld\n");
    }

    #[test]
    fn split_body_preserves_internal_content() {
        let raw = "+++\ntitle = \"T\"\n+++\nLine 1\nLine 2\n";
        let (_, body) = split_frontmatter(raw).unwrap();
        assert_eq!(body, "Line 1\nLine 2\n");
    }

    #[test]
    fn split_missing_opening_fence_errors() {
        let err = split_frontmatter("title = \"T\"\n+++\nbody\n").unwrap_err();
        assert!(matches!(err, Error::InvalidFrontmatter(_)));
    }

    #[test]
    fn split_missing_closing_fence_errors() {
        let err = split_frontmatter("+++\ntitle = \"T\"\nbody\n").unwrap_err();
        assert!(matches!(err, Error::InvalidFrontmatter(_)));
    }

    #[test]
    fn split_allows_empty_body() {
        let raw = "+++\ntitle = \"T\"\n+++\n";
        let (fm, body) = split_frontmatter(raw).unwrap();
        assert_eq!(fm, "title = \"T\"");
        assert_eq!(body, "");
    }

    #[test]
    fn split_body_without_trailing_newline() {
        let raw = "+++\ntitle = \"T\"\n+++\nbody";
        let (_, body) = split_frontmatter(raw).unwrap();
        assert_eq!(body, "body");
    }

    #[test]
    fn rejects_closing_fence_with_trailing_text() {
        let raw = "+++\ntitle = \"T\"\n+++ trailing\nbody\n";
        assert!(matches!(
            split_frontmatter(raw),
            Err(Error::InvalidFrontmatter(_))
        ));
    }

    #[test]
    fn parse_frontmatter_only_title_leaves_extras_empty() {
        let fm = parse_frontmatter("title = \"Section One\"\n").unwrap();
        assert_eq!(fm.title, "Section One");
        assert!(fm.scripts.is_empty());
        assert!(fm.styles.is_empty());
    }

    #[test]
    fn parse_frontmatter_with_scripts_and_styles() {
        let raw = "title = \"T\"\nscripts = [\"/a.js\"]\nstyles = [\"/b.css\", \"/c.css\"]\n";
        let fm = parse_frontmatter(raw).unwrap();
        assert_eq!(fm.scripts, vec!["/a.js"]);
        assert_eq!(fm.styles, vec!["/b.css", "/c.css"]);
    }

    #[test]
    fn parse_frontmatter_blank_title_errors() {
        assert!(parse_frontmatter("title = \"   \"\n").is_err());
    }

    #[test]
    fn parse_frontmatter_bad_toml_errors() {
        assert!(parse_frontmatter("not toml [").is_err());
    }
}
