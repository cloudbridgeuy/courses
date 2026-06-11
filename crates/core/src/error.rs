/// Errors produced by the pure course domain.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("course not found: {0}")]
    CourseNotFound(String),

    #[error("invalid course slug: {0:?}")]
    InvalidSlug(String),
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
}
