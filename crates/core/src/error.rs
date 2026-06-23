/// Errors produced by the pure course domain.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reserved for a future direct-lookup API; not raised by MVP 1 (the
    /// server resolves unknown slugs to its 404 page without this error).
    #[error("course not found: {0}")]
    CourseNotFound(String),

    #[error("invalid course slug: {0:?}")]
    InvalidSlug(String),

    #[error("invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("invalid frontmatter: {0}")]
    InvalidFrontmatter(String),

    #[error("section file named in manifest but absent: {0}")]
    MissingSection(String),

    #[error("unclosed `::: solucion` block")]
    UnclosedSolution,

    #[error("nested `::: solucion` block")]
    NestedSolution,

    #[error("unclosed `:::slide` block")]
    UnclosedSlide,

    #[error("nested `:::slide` block")]
    NestedSlide,

    #[error("unclosed `:::inline-slide` block")]
    UnclosedInlineSlide,

    #[error("nested `:::inline-slide` block")]
    NestedInlineSlide,

    #[error("unclosed `:::title-slide` block")]
    UnclosedTitleSlide,

    #[error("nested `:::title-slide` block")]
    NestedTitleSlide,

    #[error("`:::title-slide` block must be empty")]
    TitleSlideNotEmpty,

    #[error("unclosed `::: warning` block")]
    UnclosedWarning,

    #[error("nested `::: warning` block")]
    NestedWarning,

    #[error("unclosed `::: extra` block")]
    UnclosedExtra,

    #[error("nested `::: extra` block")]
    NestedExtra,

    #[error("unclosed `:::app` block")]
    UnclosedApp,

    #[error("nested `:::app` block")]
    NestedApp,

    #[error("duplicate anchor: {0}")]
    DuplicateAnchor(String),

    #[error("anchor {0} is not followed by a block")]
    EmptyAnchor(String),

    #[error("slide references unknown anchor: {0}")]
    UnknownSlideRef(String),

    #[error("in section {file}: {message}")]
    InvalidSection { file: String, message: String },

    #[error("malformed SNS payload: {0}")]
    MalformedNotification(String),

    #[error("malformed event: {0}")]
    MalformedEvent(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn display_course_not_found() {
        let err = Error::CourseNotFound("aws-devops".to_owned());
        assert_eq!(err.to_string(), "course not found: aws-devops");
    }

    #[test]
    fn display_invalid_slug() {
        let err = Error::InvalidSlug("Not A Slug".to_owned());
        assert_eq!(err.to_string(), "invalid course slug: \"Not A Slug\"");
    }

    #[test]
    fn display_invalid_manifest() {
        let err = Error::InvalidManifest("title must not be empty".to_owned());
        assert_eq!(err.to_string(), "invalid manifest: title must not be empty");
    }

    #[test]
    fn display_invalid_frontmatter() {
        let err = Error::InvalidFrontmatter("missing opening +++".to_owned());
        assert_eq!(err.to_string(), "invalid frontmatter: missing opening +++");
    }

    #[test]
    fn display_missing_section() {
        let err = Error::MissingSection("01-intro.md".to_owned());
        assert_eq!(
            err.to_string(),
            "section file named in manifest but absent: 01-intro.md"
        );
    }

    #[test]
    fn display_unclosed_solution() {
        let err = Error::UnclosedSolution;
        assert_eq!(err.to_string(), "unclosed `::: solucion` block");
    }

    #[test]
    fn display_nested_solution() {
        let err = Error::NestedSolution;
        assert_eq!(err.to_string(), "nested `::: solucion` block");
    }

    #[test]
    fn display_unclosed_slide() {
        assert_eq!(
            Error::UnclosedSlide.to_string(),
            "unclosed `:::slide` block"
        );
    }

    #[test]
    fn display_nested_slide() {
        assert_eq!(Error::NestedSlide.to_string(), "nested `:::slide` block");
    }

    #[test]
    fn display_unclosed_inline_slide() {
        assert_eq!(
            Error::UnclosedInlineSlide.to_string(),
            "unclosed `:::inline-slide` block"
        );
    }

    #[test]
    fn display_nested_inline_slide() {
        assert_eq!(
            Error::NestedInlineSlide.to_string(),
            "nested `:::inline-slide` block"
        );
    }

    #[test]
    fn display_unclosed_title_slide() {
        assert_eq!(
            Error::UnclosedTitleSlide.to_string(),
            "unclosed `:::title-slide` block"
        );
    }

    #[test]
    fn display_nested_title_slide() {
        assert_eq!(
            Error::NestedTitleSlide.to_string(),
            "nested `:::title-slide` block"
        );
    }

    #[test]
    fn display_title_slide_not_empty() {
        assert_eq!(
            Error::TitleSlideNotEmpty.to_string(),
            "`:::title-slide` block must be empty"
        );
    }

    #[test]
    fn display_unclosed_warning() {
        assert_eq!(
            Error::UnclosedWarning.to_string(),
            "unclosed `::: warning` block"
        );
    }

    #[test]
    fn display_nested_warning() {
        assert_eq!(
            Error::NestedWarning.to_string(),
            "nested `::: warning` block"
        );
    }

    #[test]
    fn display_unclosed_extra() {
        assert_eq!(
            Error::UnclosedExtra.to_string(),
            "unclosed `::: extra` block"
        );
    }

    #[test]
    fn display_nested_extra() {
        assert_eq!(Error::NestedExtra.to_string(), "nested `::: extra` block");
    }

    #[test]
    fn display_unclosed_app() {
        assert_eq!(Error::UnclosedApp.to_string(), "unclosed `:::app` block");
    }

    #[test]
    fn display_nested_app() {
        assert_eq!(Error::NestedApp.to_string(), "nested `:::app` block");
    }

    #[test]
    fn display_duplicate_anchor() {
        assert_eq!(
            Error::DuplicateAnchor("tabla".to_owned()).to_string(),
            "duplicate anchor: tabla"
        );
    }

    #[test]
    fn display_empty_anchor() {
        assert_eq!(
            Error::EmptyAnchor("tabla".to_owned()).to_string(),
            "anchor tabla is not followed by a block"
        );
    }

    #[test]
    fn display_unknown_slide_ref() {
        assert_eq!(
            Error::UnknownSlideRef("tabla".to_owned()).to_string(),
            "slide references unknown anchor: tabla"
        );
    }

    #[test]
    fn display_invalid_section_names_the_file() {
        let err = Error::InvalidSection {
            file: "01-intro.md".to_owned(),
            message: "unclosed `::: solucion` block".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "in section 01-intro.md: unclosed `::: solucion` block"
        );
    }
}
