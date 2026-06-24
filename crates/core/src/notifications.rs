//! Pure parsing of AWS SNS HTTP(S) payloads into a domain `Notification`.
//!
//! No I/O lives here: the shell performs the subscription-confirmation GET and
//! the SSE fan-out. This module only turns a raw request body into a typed value.
//!
//! Two SNS message kinds matter:
//! - `SubscriptionConfirmation` — carries a `SubscribeURL` the shell must GET.
//! - `Notification` — wraps the actual event in a `Message` string, which is
//!   itself JSON (an EventBridge or CodeStar Notifications payload).

use serde::Deserialize;
use serde_json::Value;

use crate::error::{Error, Result};

/// A pipeline/build event surfaced to the guide as a toast, attributed to a pod.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Notification {
    /// The participant the event belongs to (baked-in `pod`, else AWS account).
    pub pod: String,
    /// Event origin, e.g. `aws.codepipeline` or a `detail-type` string.
    pub source: String,
    /// State word, e.g. `SUCCEEDED`, `FAILED`, `IN_PROGRESS` (may be empty).
    pub state: String,
    /// Human-readable subject, e.g. the pipeline or project name.
    pub detail: String,
}

/// The SNS message kinds the endpoint handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnsMessage {
    /// Subscription handshake; the shell must GET `subscribe_url` to confirm.
    Confirmation { subscribe_url: String },
    /// A delivered event, parsed into a domain `Notification`.
    Event(Notification),
}

/// The outer SNS envelope. Only the fields we act on are captured.
#[derive(Deserialize)]
struct Envelope {
    #[serde(rename = "Type")]
    kind: String,
    #[serde(rename = "SubscribeURL")]
    subscribe_url: Option<String>,
    #[serde(rename = "Message")]
    message: Option<String>,
}

/// Parses a raw SNS request body into a [`SnsMessage`].
///
/// `SubscriptionConfirmation` yields [`SnsMessage::Confirmation`]; `Notification`
/// yields [`SnsMessage::Event`] with the inner event parsed. Any other envelope
/// type, or unparseable JSON, is a [`Error::MalformedNotification`].
pub fn parse_sns_message(body: &str) -> Result<SnsMessage> {
    let envelope: Envelope = serde_json::from_str(body)
        .map_err(|e| Error::MalformedNotification(format!("envelope: {e}")))?;

    match envelope.kind.as_str() {
        "SubscriptionConfirmation" => {
            let url = envelope.subscribe_url.ok_or_else(|| {
                Error::MalformedNotification("SubscriptionConfirmation without SubscribeURL".into())
            })?;
            Ok(SnsMessage::Confirmation { subscribe_url: url })
        }
        "Notification" => {
            let message = envelope.message.unwrap_or_default();
            Ok(SnsMessage::Event(notification_from_message(&message)))
        }
        other => Err(Error::MalformedNotification(format!(
            "unsupported SNS message type: {other}"
        ))),
    }
}

// ── Pure event extraction ────────────────────────────────────────────────

/// Builds a [`Notification`] from the SNS `Message` string. The message is
/// usually an EventBridge/CodeStar JSON document; when it is plain text (or
/// unparseable), the whole string becomes the detail with neutral defaults.
fn notification_from_message(message: &str) -> Notification {
    let Ok(value) = serde_json::from_str::<Value>(message) else {
        return Notification {
            pod: "desconocido".to_owned(),
            source: "sns".to_owned(),
            state: String::new(),
            detail: message.trim().to_owned(),
        };
    };

    Notification {
        pod: extract_pod(&value),
        source: first_str(&value, &["source", "detailType", "detail-type"])
            .unwrap_or("evento")
            .to_owned(),
        state: extract_state(&value),
        detail: extract_detail(&value),
    }
}

/// Resolves the pod: a baked-in `pod` (top level or under `detail`) wins, then
/// the AWS `account`, then a visible fallback so misconfiguration shows up.
fn extract_pod(value: &Value) -> String {
    if let Some(pod) = value.get("pod").and_then(Value::as_str) {
        return pod.to_owned();
    }
    if let Some(pod) = value.pointer("/detail/pod").and_then(Value::as_str) {
        return pod.to_owned();
    }
    if let Some(account) = value.get("account").and_then(Value::as_str) {
        return account.to_owned();
    }
    "desconocido".to_owned()
}

/// The event state, from the shapes CodePipeline and CodeBuild emit.
fn extract_state(value: &Value) -> String {
    for path in ["/detail/state", "/detail/build-status", "/detail/status"] {
        if let Some(state) = value.pointer(path).and_then(Value::as_str) {
            return state.to_owned();
        }
    }
    String::new()
}

/// A human subject for the toast: the pipeline or project name, else the type.
fn extract_detail(value: &Value) -> String {
    for path in ["/detail/pipeline", "/detail/project-name"] {
        if let Some(name) = value.pointer(path).and_then(Value::as_str) {
            return name.to_owned();
        }
    }
    first_str(value, &["detailType", "detail-type"])
        .unwrap_or("")
        .to_owned()
}

/// First of `keys` present as a string on `value`.
fn first_str<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|k| value.get(*k).and_then(Value::as_str))
}

// ── Demo notification builder (pure) ───────────────────────────────────────

/// Pod label used for guide-driven demo toasts, so they read as a sample rather
/// than a real participant.
const DEMO_POD: &str = "demo";

/// Builds a demo [`Notification`] from an opaque seed, cycling deterministically
/// across the three pipeline events the real notification rule selects:
/// `SUCCEEDED` (green), `FAILED` (red), and a pending manual approval (info).
///
/// The seed is the emitting event's id; equal seeds yield equal notifications.
/// This is the pure core of the `toast-demo` app: the shell only broadcasts the
/// result on the SSE bus, so demo toasts render through the same path real SNS
/// events do.
pub fn demo_notification(seed: &str) -> Notification {
    let pick = seed
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_add(u32::from(b)))
        % 3;
    match pick {
        0 => Notification {
            pod: DEMO_POD.to_owned(),
            source: "aws.codepipeline".to_owned(),
            state: "SUCCEEDED".to_owned(),
            detail: "taller-pipeline".to_owned(),
        },
        1 => Notification {
            pod: DEMO_POD.to_owned(),
            source: "aws.codebuild".to_owned(),
            state: "FAILED".to_owned(),
            detail: "taller-build".to_owned(),
        },
        _ => Notification {
            pod: DEMO_POD.to_owned(),
            source: "aws.codepipeline".to_owned(),
            state: "IN_PROGRESS".to_owned(),
            detail: "Aprobación pendiente".to_owned(),
        },
    }
}

// ── Shared-secret webhook auth ─────────────────────────────────────────────

/// Compares a provided webhook token against the expected one without an
/// early-exit on the first differing byte.
///
/// This emulates the real-world pattern chat webhooks (Slack/Teams) use: an
/// unguessable secret travels in the endpoint URL. It is deliberately simple —
/// good enough to represent the production situation in the lab, not a hardened
/// signature scheme. The length comparison still leaks length, which is fine here.
pub fn token_matches(expected: &str, provided: &str) -> bool {
    if expected.len() != provided.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in expected.bytes().zip(provided.bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn subscription_confirmation_yields_subscribe_url() {
        let body = r#"{"Type":"SubscriptionConfirmation",
            "SubscribeURL":"https://sns.us-east-1.amazonaws.com/?Action=Confirm&Token=abc"}"#;
        let parsed = parse_sns_message(body).unwrap();
        assert_eq!(
            parsed,
            SnsMessage::Confirmation {
                subscribe_url: "https://sns.us-east-1.amazonaws.com/?Action=Confirm&Token=abc"
                    .to_owned()
            }
        );
    }

    #[test]
    fn confirmation_without_url_is_malformed() {
        let body = r#"{"Type":"SubscriptionConfirmation"}"#;
        assert!(matches!(
            parse_sns_message(body),
            Err(Error::MalformedNotification(_))
        ));
    }

    #[test]
    fn eventbridge_codepipeline_event_with_pod() {
        let inner = r#"{"pod":"maria","source":"aws.codepipeline",
            "detail-type":"CodePipeline Pipeline Execution State Change",
            "account":"123456789012",
            "detail":{"pipeline":"taller-aws-maria-pipeline","state":"SUCCEEDED"}}"#;
        let body = serde_json::json!({"Type":"Notification","Message": inner}).to_string();
        let parsed = parse_sns_message(&body).unwrap();
        assert_eq!(
            parsed,
            SnsMessage::Event(Notification {
                pod: "maria".to_owned(),
                source: "aws.codepipeline".to_owned(),
                state: "SUCCEEDED".to_owned(),
                detail: "taller-aws-maria-pipeline".to_owned(),
            })
        );
    }

    #[test]
    fn pod_falls_back_to_account_when_absent() {
        let inner = r#"{"source":"aws.codebuild","account":"123456789012",
            "detail":{"project-name":"taller-build","build-status":"FAILED"}}"#;
        let body = serde_json::json!({"Type":"Notification","Message": inner}).to_string();
        let SnsMessage::Event(n) = parse_sns_message(&body).unwrap() else {
            panic!("expected event");
        };
        assert_eq!(n.pod, "123456789012");
        assert_eq!(n.state, "FAILED");
        assert_eq!(n.detail, "taller-build");
    }

    #[test]
    fn plain_text_message_becomes_detail() {
        let body = serde_json::json!({"Type":"Notification","Message":"hola mundo"}).to_string();
        let SnsMessage::Event(n) = parse_sns_message(&body).unwrap() else {
            panic!("expected event");
        };
        assert_eq!(n.pod, "desconocido");
        assert_eq!(n.detail, "hola mundo");
    }

    #[test]
    fn unsupported_type_is_malformed() {
        let body = r#"{"Type":"UnsubscribeConfirmation","SubscribeURL":"x"}"#;
        assert!(matches!(
            parse_sns_message(body),
            Err(Error::MalformedNotification(_))
        ));
    }

    #[test]
    fn garbage_body_is_malformed() {
        assert!(matches!(
            parse_sns_message("not json"),
            Err(Error::MalformedNotification(_))
        ));
    }

    #[test]
    fn demo_notification_is_deterministic() {
        assert_eq!(demo_notification("abc-123"), demo_notification("abc-123"));
    }

    #[test]
    fn demo_notification_cycles_three_states() {
        // Seeds "0", "1", "2" (bytes 48, 49, 50) hit picks 0, 1, 2.
        assert_eq!(demo_notification("0").state, "SUCCEEDED");
        assert_eq!(demo_notification("1").state, "FAILED");
        assert_eq!(demo_notification("2").state, "IN_PROGRESS");
    }

    #[test]
    fn demo_notification_succeeded_variant() {
        let n = demo_notification("0");
        assert_eq!(n.source, "aws.codepipeline");
        assert_eq!(n.detail, "taller-pipeline");
    }

    #[test]
    fn demo_notification_failed_variant() {
        let n = demo_notification("1");
        assert_eq!(n.source, "aws.codebuild");
        assert_eq!(n.detail, "taller-build");
    }

    #[test]
    fn demo_notification_always_uses_demo_pod() {
        for seed in ["0", "1", "2", "anything"] {
            assert_eq!(demo_notification(seed).pod, "demo");
        }
    }

    #[test]
    fn token_matches_only_on_exact_equality() {
        assert!(token_matches("s3cr3t", "s3cr3t"));
        assert!(!token_matches("s3cr3t", "s3cr3T"));
        assert!(!token_matches("s3cr3t", "s3cr3t-extra"));
        assert!(!token_matches("s3cr3t", ""));
    }
}
