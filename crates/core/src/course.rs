use serde::Serialize;

use crate::error::{Error, Result};

fn is_kebab_slug(raw: &str) -> bool {
    !raw.is_empty()
        && raw
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !raw.starts_with('-')
        && !raw.ends_with('-')
        && !raw.contains("--")
}

/// Lowercase-kebab identifier for a course (e.g. `aws-devops`).
///
/// The only constructor is [`CourseSlug::parse`]; a value of this type is
/// always a valid slug (parse, don't validate).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct CourseSlug(String);

impl CourseSlug {
    /// Parses a raw string into a slug.
    ///
    /// Valid slugs are non-empty ASCII lowercase alphanumeric segments,
    /// separated by single hyphens, with no leading, or trailing, hyphen.
    pub fn parse(raw: &str) -> Result<Self> {
        if is_kebab_slug(raw) {
            Ok(Self(raw.to_owned()))
        } else {
            Err(Error::InvalidSlug(raw.to_owned()))
        }
    }

    /// Returns the slug as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Lowercase-kebab identifier for a session within a course, unique per course
/// (e.g. `del-codigo-a-la-imagen`). The only constructor is [`SessionSlug::parse`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SessionSlug(String);

impl SessionSlug {
    /// Parses a raw string into a session slug (same form as a course slug).
    pub fn parse(raw: &str) -> Result<Self> {
        if is_kebab_slug(raw) {
            Ok(Self(raw.to_owned()))
        } else {
            Err(Error::InvalidSlug(raw.to_owned()))
        }
    }

    /// Returns the slug as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One titled section of a course guide.
///
/// `body_html` is trusted, authored HTML; it is rendered verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GuideSection {
    pub title: String,
    pub body_html: String,
}

/// One session of a course: its slug, its title, and its ordered sections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Session {
    pub slug: SessionSlug,
    pub title: String,
    pub sections: Vec<GuideSection>,
}

/// A course: its slug, its title, and its sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Course {
    pub slug: CourseSlug,
    pub title: String,
    pub sessions: Vec<Session>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_simple_slug() {
        assert_eq!(
            CourseSlug::parse("aws-devops").unwrap().as_str(),
            "aws-devops"
        );
    }

    #[test]
    fn parse_accepts_single_segment_and_digits() {
        assert!(CourseSlug::parse("a").is_ok());
        assert!(CourseSlug::parse("week-1").is_ok());
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(matches!(
            CourseSlug::parse(""),
            Err(crate::Error::InvalidSlug(_))
        ));
    }

    #[test]
    fn parse_rejects_uppercase() {
        assert!(CourseSlug::parse("AWS").is_err());
    }

    #[test]
    fn parse_rejects_bad_hyphens() {
        assert!(CourseSlug::parse("-a").is_err());
        assert!(CourseSlug::parse("a-").is_err());
        assert!(CourseSlug::parse("a--b").is_err());
    }

    #[test]
    fn parse_rejects_invalid_characters() {
        assert!(CourseSlug::parse("a_b").is_err());
        assert!(CourseSlug::parse("café").is_err());
        assert!(CourseSlug::parse("a b").is_err());
    }

    #[test]
    fn parse_accepts_multi_segment_slug() {
        assert_eq!(CourseSlug::parse("a-b-c").unwrap().as_str(), "a-b-c");
    }

    #[test]
    fn parse_accepts_digits_only() {
        assert!(CourseSlug::parse("123").is_ok());
    }

    #[test]
    fn session_slug_accepts_valid_slugs() {
        assert_eq!(
            SessionSlug::parse("del-codigo-a-la-imagen")
                .unwrap()
                .as_str(),
            "del-codigo-a-la-imagen"
        );
        assert!(SessionSlug::parse("a").is_ok());
        assert!(SessionSlug::parse("week-1").is_ok());
    }

    #[test]
    fn session_slug_rejects_empty() {
        assert!(matches!(
            SessionSlug::parse(""),
            Err(crate::Error::InvalidSlug(_))
        ));
    }

    #[test]
    fn session_slug_rejects_uppercase() {
        assert!(SessionSlug::parse("Sesion").is_err());
    }

    #[test]
    fn session_slug_rejects_bad_hyphens() {
        assert!(SessionSlug::parse("-a").is_err());
        assert!(SessionSlug::parse("a-").is_err());
        assert!(SessionSlug::parse("a--b").is_err());
    }

    #[test]
    fn session_slug_rejects_non_kebab_characters() {
        assert!(SessionSlug::parse("a_b").is_err());
        assert!(SessionSlug::parse("café").is_err());
        assert!(SessionSlug::parse("a b").is_err());
    }
}
