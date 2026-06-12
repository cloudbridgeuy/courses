use serde::Deserialize;

use crate::error::{Error, Result};

/// One session entry in a course manifest.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SessionEntry {
    pub slug: String,
    pub title: String,
    pub sections: Vec<String>,
}

/// A course manifest, as authored in `content/<slug>/course.toml`.
///
/// `session` lists the course's sessions in display order; each names its
/// section files in display order.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Manifest {
    pub title: String,
    #[serde(rename = "session", default)]
    pub sessions: Vec<SessionEntry>,
}

/// Parses raw TOML into a [`Manifest`] (parse, don't validate).
pub fn parse_manifest(raw: &str) -> Result<Manifest> {
    let manifest: Manifest =
        toml::from_str(raw).map_err(|e| Error::InvalidManifest(e.to_string()))?;
    if manifest.title.trim().is_empty() {
        return Err(Error::InvalidManifest("title must not be empty".to_owned()));
    }
    if manifest.sessions.is_empty() {
        return Err(Error::InvalidManifest(
            "a course must have at least one session".to_owned(),
        ));
    }
    for session in &manifest.sessions {
        if session.title.trim().is_empty() {
            return Err(Error::InvalidManifest(
                "session title must not be empty".to_owned(),
            ));
        }
        if session.sections.is_empty() {
            return Err(Error::InvalidManifest(format!(
                "session {:?} has no sections",
                session.slug
            )));
        }
    }
    Ok(manifest)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    const VALID_TWO_SESSION: &str = r#"
title = "My Course"

[[session]]
slug = "s1"
title = "Session One"
sections = ["01-a.md", "02-b.md"]

[[session]]
slug = "s2"
title = "Session Two"
sections = ["03-c.md"]
"#;

    #[test]
    fn parses_valid_two_session_manifest() {
        let m = parse_manifest(VALID_TWO_SESSION).unwrap();
        assert_eq!(m.title, "My Course");
        assert_eq!(m.sessions.len(), 2);
        assert_eq!(m.sessions[0].slug, "s1");
        assert_eq!(m.sessions[0].title, "Session One");
        assert_eq!(m.sessions[0].sections, vec!["01-a.md", "02-b.md"]);
        assert_eq!(m.sessions[1].slug, "s2");
        assert_eq!(m.sessions[1].title, "Session Two");
        assert_eq!(m.sessions[1].sections, vec!["03-c.md"]);
    }

    #[test]
    fn rejects_empty_session_set() {
        assert!(matches!(
            parse_manifest("title = \"My Course\"\n"),
            Err(Error::InvalidManifest(_))
        ));
    }

    #[test]
    fn rejects_session_with_empty_sections() {
        let toml = r#"
title = "My Course"

[[session]]
slug = "s1"
title = "Session One"
sections = []
"#;
        assert!(matches!(
            parse_manifest(toml),
            Err(Error::InvalidManifest(_))
        ));
    }

    #[test]
    fn rejects_blank_course_title() {
        let toml = r#"
title = "   "

[[session]]
slug = "s1"
title = "Session One"
sections = ["01-a.md"]
"#;
        assert!(matches!(
            parse_manifest(toml),
            Err(Error::InvalidManifest(_))
        ));
    }

    #[test]
    fn rejects_blank_session_title() {
        let toml = r#"
title = "My Course"

[[session]]
slug = "s1"
title = "   "
sections = ["01-a.md"]
"#;
        assert!(matches!(
            parse_manifest(toml),
            Err(Error::InvalidManifest(_))
        ));
    }

    #[test]
    fn rejects_missing_title_key() {
        let toml = r#"
[[session]]
slug = "s1"
title = "Session One"
sections = ["01-a.md"]
"#;
        assert!(matches!(
            parse_manifest(toml),
            Err(Error::InvalidManifest(_))
        ));
    }

    #[test]
    fn rejects_non_toml() {
        assert!(parse_manifest("not toml at all [").is_err());
    }
}
