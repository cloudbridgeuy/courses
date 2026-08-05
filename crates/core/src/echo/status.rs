//! The status code of an echo answer, chosen by the query string.
//!
//! An echo server that always answers `200` can only demonstrate the happy
//! path. Letting the caller ask for a code — `/eco/prueba?status=503` — turns it
//! into a target for anything that reacts to failure: a load balancer health
//! check, an alarm, a retry policy.

use serde_json::{Value, json};

use super::parse_query;

/// Query parameter that chooses the status code of the answer, for example
/// `/eco/prueba?status=503`.
pub const STATUS_QUERY_KEY: &str = "status";

/// Status code answered when the request does not ask for another one.
pub const DEFAULT_ECHO_STATUS: u16 = 200;

/// Smallest status code a request may ask for. Below it are the informational
/// codes, which are not final answers.
pub const MIN_ECHO_STATUS: u16 = 200;

/// Largest status code a request may ask for.
pub const MAX_ECHO_STATUS: u16 = 599;

/// What the query string asked the status code of the answer to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EchoStatus {
    /// No `status` parameter: the answer carries [`DEFAULT_ECHO_STATUS`].
    Default,
    /// A usable code, between [`MIN_ECHO_STATUS`] and [`MAX_ECHO_STATUS`].
    Requested(u16),
    /// A `status` parameter that is not a usable code. The answer falls back to
    /// the default, and reports what arrived, rather than refusing the request:
    /// a mistyped status must still show a request the caller can read.
    Invalid(String),
}

impl EchoStatus {
    /// The status code to write.
    #[must_use]
    pub fn code(&self) -> u16 {
        match self {
            Self::Requested(code) => *code,
            Self::Default | Self::Invalid(_) => DEFAULT_ECHO_STATUS,
        }
    }
}

/// Reads the status code the query string asks for.
///
/// A repeated `status` keeps the first value, which is the one a reader of the
/// URL sees first. Anything that is not a whole number in range is
/// [`EchoStatus::Invalid`].
pub fn echo_status(query: Option<&str>) -> EchoStatus {
    let parsed = parse_query(query.unwrap_or_default());
    let Some(raw) = parsed.get(STATUS_QUERY_KEY).and_then(|v| v.first()) else {
        return EchoStatus::Default;
    };
    match raw.trim().parse::<u16>() {
        Ok(code) if (MIN_ECHO_STATUS..=MAX_ECHO_STATUS).contains(&code) => {
            EchoStatus::Requested(code)
        }
        _ => EchoStatus::Invalid(raw.clone()),
    }
}

/// What status code the answer carries, and why.
///
/// It is reported in the body because the status of an answer is easy to miss:
/// a browser, or a `curl` without `-i`, shows the body and hides the code.
pub(super) fn status_json(status: &EchoStatus) -> Value {
    match status {
        EchoStatus::Default => json!({
            "status": DEFAULT_ECHO_STATUS,
            "source": "default",
            "requested": Value::Null,
        }),
        EchoStatus::Requested(code) => json!({
            "status": code,
            "source": "query",
            "requested": code.to_string(),
        }),
        EchoStatus::Invalid(raw) => json!({
            "status": DEFAULT_ECHO_STATUS,
            "source": "default",
            "requested": raw,
            "error": format!(
                "`{STATUS_QUERY_KEY}` must be a whole number between {MIN_ECHO_STATUS} and {MAX_ECHO_STATUS}"
            ),
        }),
    }
}

// --- Tests -----------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::{DEFAULT_ECHO_STATUS, EchoStatus, echo_status, status_json};
    use serde_json::{Value, json};

    #[test]
    fn answers_the_default_status_without_a_status_parameter() {
        assert_eq!(echo_status(None), EchoStatus::Default);
        assert_eq!(echo_status(Some("x=1")), EchoStatus::Default);
        assert_eq!(EchoStatus::Default.code(), DEFAULT_ECHO_STATUS);
    }

    #[test]
    fn reads_the_status_the_query_string_asks_for() {
        assert_eq!(echo_status(Some("status=503")), EchoStatus::Requested(503));
        assert_eq!(
            echo_status(Some("x=1&status=404&y=2")),
            EchoStatus::Requested(404)
        );
        assert_eq!(EchoStatus::Requested(503).code(), 503);
    }

    #[test]
    fn accepts_the_ends_of_the_status_range() {
        assert_eq!(echo_status(Some("status=200")), EchoStatus::Requested(200));
        assert_eq!(echo_status(Some("status=599")), EchoStatus::Requested(599));
    }

    #[test]
    fn refuses_a_status_outside_the_range() {
        assert_eq!(
            echo_status(Some("status=199")),
            EchoStatus::Invalid("199".to_owned())
        );
        assert_eq!(
            echo_status(Some("status=600")),
            EchoStatus::Invalid("600".to_owned())
        );
        // Over `u16`, so it does not even parse as a number.
        assert_eq!(
            echo_status(Some("status=70000")),
            EchoStatus::Invalid("70000".to_owned())
        );
    }

    #[test]
    fn refuses_a_status_that_is_not_a_whole_number() {
        assert_eq!(
            echo_status(Some("status=abc")),
            EchoStatus::Invalid("abc".to_owned())
        );
        assert_eq!(
            echo_status(Some("status=503.5")),
            EchoStatus::Invalid("503.5".to_owned())
        );
        assert_eq!(
            echo_status(Some("status")),
            EchoStatus::Invalid(String::new())
        );
        assert_eq!(EchoStatus::Invalid("abc".to_owned()).code(), 200);
    }

    #[test]
    fn keeps_the_first_of_a_repeated_status() {
        assert_eq!(
            echo_status(Some("status=418&status=500")),
            EchoStatus::Requested(418)
        );
    }

    #[test]
    fn ignores_the_space_around_a_status() {
        assert_eq!(
            echo_status(Some("status=%20503")),
            EchoStatus::Requested(503)
        );
    }

    #[test]
    fn reports_where_the_status_came_from() {
        assert_eq!(
            status_json(&EchoStatus::Requested(503)),
            json!({ "status": 503, "source": "query", "requested": "503" })
        );
        let default = status_json(&EchoStatus::Default);
        assert_eq!(default["source"], json!("default"));
        assert_eq!(default["requested"], Value::Null);
    }

    #[test]
    fn explains_a_status_it_could_not_use() {
        let invalid = status_json(&EchoStatus::Invalid("teapot".to_owned()));
        assert_eq!(invalid["status"], json!(DEFAULT_ECHO_STATUS));
        assert_eq!(invalid["requested"], json!("teapot"));
        assert!(
            invalid["error"]
                .as_str()
                .is_some_and(|error| error.contains("status"))
        );
    }
}
