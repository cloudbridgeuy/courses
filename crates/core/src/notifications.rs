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
///
/// The first four fields are what every toast shows; the rest answer *what the
/// event was about* and are empty whenever the payload does not carry them.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct Notification {
    /// The participant the event belongs to (baked-in `pod`, else AWS account).
    pub pod: String,
    /// Event origin, e.g. `aws.codepipeline` or a `detail-type` string.
    pub source: String,
    /// State word, e.g. `SUCCEEDED`, `FAILED`, `IN_PROGRESS` (may be empty).
    pub state: String,
    /// Human-readable subject, e.g. the pipeline or project name.
    pub detail: String,
    /// Pipeline stage, on stage- and action-level events.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub stage: String,
    /// Pipeline action, on action-level events.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub action: String,
    /// The action's provider (`CodeBuild`, `CloudFormation`, `Manual`, …).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub provider: String,
    /// CodeBuild phase, e.g. `BUILD`, `POST_BUILD`, `COMPLETED`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub phase: String,
    /// Why it happened: action failure summary, error code, or alarm reason.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reason: String,
    /// Pipeline execution id, or CodeBuild build id.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub execution: String,
    /// AWS region the event came from.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub region: String,
    /// When AWS published the event (as published; RFC 3339 in practice).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub time: String,
    /// Console deep link for the subject, when one can be built.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub url: String,
}

/// The SNS message kinds the endpoint handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnsMessage {
    /// Subscription handshake; the shell must GET `subscribe_url` to confirm.
    Confirmation { subscribe_url: String },
    /// A delivered event, parsed into a domain `Notification`. Boxed because a
    /// `Notification` is an order of magnitude bigger than a subscribe URL.
    Event(Box<Notification>),
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
            Ok(SnsMessage::Event(Box::new(notification_from_message(
                &message,
            ))))
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
            detail: message.trim().to_owned(),
            ..Notification::default()
        };
    };

    let region = extract_region(&value);
    let mut event = Notification {
        pod: extract_pod(&value),
        source: extract_source(&value),
        state: extract_state(&value),
        detail: extract_detail(&value),
        stage: pointer_str(&value, "/detail/stage"),
        action: pointer_str(&value, "/detail/action"),
        provider: pointer_str(&value, "/detail/type/provider"),
        phase: first_of(
            &value,
            &["/detail/current-phase", "/detail/completed-phase"],
        ),
        reason: extract_reason(&value),
        execution: extract_execution(&value),
        region,
        time: first_of(&value, &["/time", "/StateChangeTime"]),
        url: String::new(),
    };
    event.url = console_url(&value, &event);
    event
}

/// The event origin. A CloudWatch alarm carries no `source`, so its shape
/// (`AlarmName`) names the service instead of falling through to `evento`.
fn extract_source(value: &Value) -> String {
    if let Some(source) = first_str(value, &["source", "detailType", "detail-type"]) {
        return source.to_owned();
    }
    if value.get("AlarmName").is_some() {
        return "aws.cloudwatch".to_owned();
    }
    "evento".to_owned()
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
    if let Some(account) = first_str(value, &["account", "AWSAccountId"]) {
        return account.to_owned();
    }
    "desconocido".to_owned()
}

/// The event state, from the shapes CodePipeline, CodeBuild, and a CloudWatch
/// alarm (`NewStateValue`) emit.
fn extract_state(value: &Value) -> String {
    for path in [
        "/detail/state",
        "/detail/build-status",
        "/detail/status",
        "/NewStateValue",
    ] {
        if let Some(state) = value.pointer(path).and_then(Value::as_str) {
            return state.to_owned();
        }
    }
    String::new()
}

/// A human subject for the toast: the pipeline, project, or alarm name, else
/// the type.
fn extract_detail(value: &Value) -> String {
    for path in ["/detail/pipeline", "/detail/project-name"] {
        if let Some(name) = value.pointer(path).and_then(Value::as_str) {
            return name.to_owned();
        }
    }
    first_str(value, &["AlarmName", "detailType", "detail-type"])
        .unwrap_or("")
        .to_owned()
}

/// Why the event happened: an action's failure summary, its error code, or the
/// reason a CloudWatch alarm changed state.
fn extract_reason(value: &Value) -> String {
    first_of(
        value,
        &[
            "/detail/execution-result/external-execution-summary",
            "/detail/execution-result/error-code",
            "/NewStateReason",
            "/detail/additional-information/message",
        ],
    )
}

/// The pipeline execution, or the CodeBuild build, the event belongs to.
fn extract_execution(value: &Value) -> String {
    let id = first_of(
        value,
        &[
            "/detail/execution-id",
            "/detail/build-id",
            "/detail/execution-result/external-execution-id",
        ],
    );
    // A build id arrives as a full ARN; only the tail identifies the build.
    match id.split_once("build/") {
        Some((_, tail)) => tail.to_owned(),
        None => id,
    }
}

/// The region: the event's own field, else the one inside the first ARN it
/// names (alarms carry no `region`, only `AlarmArn`).
fn extract_region(value: &Value) -> String {
    if let Some(region) = value.get("region").and_then(Value::as_str) {
        return region.to_owned();
    }
    let arn = value
        .pointer("/resources/0")
        .and_then(Value::as_str)
        .or_else(|| value.get("AlarmArn").and_then(Value::as_str))
        .unwrap_or_default();
    // arn:aws:<service>:<region>:<account>:…
    arn.split(':').nth(3).unwrap_or_default().to_owned()
}

/// A console deep link for the subject. An action that ran elsewhere publishes
/// its own URL; otherwise the link is built from region, name, and execution.
/// Returns an empty string when the pieces for a valid link are missing.
fn console_url(value: &Value, event: &Notification) -> String {
    let external = pointer_str(value, "/detail/execution-result/external-execution-url");
    if !external.is_empty() {
        return external;
    }
    let deep_link = pointer_str(value, "/detail/additional-information/logs/deep-link");
    if !deep_link.is_empty() {
        return deep_link;
    }
    if event.region.is_empty() || event.detail.is_empty() {
        return String::new();
    }
    let base = format!("https://{}.console.aws.amazon.com", event.region);
    let name = &event.detail;
    match event.source.as_str() {
        "aws.codepipeline" if event.execution.is_empty() => {
            format!(
                "{base}/codesuite/codepipeline/pipelines/{name}/view?region={}",
                event.region
            )
        }
        "aws.codepipeline" => format!(
            "{base}/codesuite/codepipeline/pipelines/{name}/executions/{}?region={}",
            event.execution, event.region
        ),
        "aws.codebuild" if event.execution.is_empty() => format!(
            "{base}/codesuite/codebuild/projects/{name}/history?region={}",
            event.region
        ),
        "aws.codebuild" => format!(
            "{base}/codesuite/codebuild/projects/{name}/build/{}?region={}",
            event.execution, event.region
        ),
        "aws.cloudwatch" => format!(
            "{base}/cloudwatch/home?region={}#alarmsV2:alarm/{}",
            event.region,
            name.replace(' ', "%20")
        ),
        _ => String::new(),
    }
}

/// The string at `path`, or empty.
fn pointer_str(value: &Value, path: &str) -> String {
    value
        .pointer(path)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// First of `paths` present as a string on `value`, or empty.
fn first_of(value: &Value, paths: &[&str]) -> String {
    paths
        .iter()
        .find_map(|p| value.pointer(p).and_then(Value::as_str))
        .unwrap_or_default()
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
    // No `url`: a demo toast must not send anyone to a console page that does
    // not exist. Everything else mirrors the shape of a real event.
    match pick {
        0 => Notification {
            pod: DEMO_POD.to_owned(),
            source: "aws.codepipeline".to_owned(),
            state: "SUCCEEDED".to_owned(),
            detail: "taller-pipeline".to_owned(),
            execution: "e1a2b3c4-5d6e-7f80-9012-3456789abcde".to_owned(),
            region: "us-east-2".to_owned(),
            ..Notification::default()
        },
        1 => Notification {
            pod: DEMO_POD.to_owned(),
            source: "aws.codebuild".to_owned(),
            state: "FAILED".to_owned(),
            detail: "taller-build".to_owned(),
            phase: "BUILD".to_owned(),
            reason: "COMMAND_EXECUTION_ERROR: Error while executing command: docker build"
                .to_owned(),
            region: "us-east-2".to_owned(),
            ..Notification::default()
        },
        _ => Notification {
            pod: DEMO_POD.to_owned(),
            source: "aws.codepipeline".to_owned(),
            state: "IN_PROGRESS".to_owned(),
            detail: "Aprobación pendiente".to_owned(),
            stage: "Aprobacion".to_owned(),
            action: "Revisar".to_owned(),
            provider: "Manual".to_owned(),
            region: "us-east-2".to_owned(),
            ..Notification::default()
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
            SnsMessage::Event(Box::new(Notification {
                pod: "maria".to_owned(),
                source: "aws.codepipeline".to_owned(),
                state: "SUCCEEDED".to_owned(),
                detail: "taller-aws-maria-pipeline".to_owned(),
                ..Notification::default()
            }))
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
    fn cloudwatch_alarm_fills_every_field() {
        let inner = r#"{"AlarmName":"taller-cpu-alta","NewStateValue":"ALARM",
            "AWSAccountId":"123456789012",
            "AlarmArn":"arn:aws:cloudwatch:us-east-2:123456789012:alarm:taller-cpu-alta",
            "StateChangeTime":"2026-08-04T18:31:02.000+0000",
            "NewStateReason":"Threshold Crossed: 1 datapoint [85.0] > 80.0"}"#;
        let body = serde_json::json!({"Type":"Notification","Message": inner}).to_string();
        assert_eq!(
            parse_sns_message(&body).unwrap(),
            SnsMessage::Event(Box::new(Notification {
                pod: "123456789012".to_owned(),
                source: "aws.cloudwatch".to_owned(),
                state: "ALARM".to_owned(),
                detail: "taller-cpu-alta".to_owned(),
                reason: "Threshold Crossed: 1 datapoint [85.0] > 80.0".to_owned(),
                region: "us-east-2".to_owned(),
                time: "2026-08-04T18:31:02.000+0000".to_owned(),
                url: "https://us-east-2.console.aws.amazon.com/cloudwatch/home\
                      ?region=us-east-2#alarmsV2:alarm/taller-cpu-alta"
                    .to_owned(),
                ..Notification::default()
            }))
        );
    }

    #[test]
    fn action_level_failure_carries_stage_action_and_reason() {
        let inner = r#"{"source":"aws.codepipeline","account":"123456789012",
            "region":"us-east-2","time":"2026-08-04T18:31:02Z",
            "detailType":"CodePipeline Action Execution State Change",
            "detail":{"pipeline":"taller-pipeline","execution-id":"e1a2b3c4",
                "stage":"Build","action":"Compilar","state":"FAILED",
                "type":{"owner":"AWS","provider":"CodeBuild","category":"Build"},
                "execution-result":{"error-code":"JobFailed",
                    "external-execution-summary":"Build terminó con exit 1",
                    "external-execution-url":"https://console/build/42"}}}"#;
        let body = serde_json::json!({"Type":"Notification","Message": inner}).to_string();
        let SnsMessage::Event(n) = parse_sns_message(&body).unwrap() else {
            panic!("expected event");
        };
        assert_eq!(n.stage, "Build");
        assert_eq!(n.action, "Compilar");
        assert_eq!(n.provider, "CodeBuild");
        assert_eq!(n.reason, "Build terminó con exit 1");
        assert_eq!(n.execution, "e1a2b3c4");
        assert_eq!(n.time, "2026-08-04T18:31:02Z");
        // The action's own URL wins over anything we could build.
        assert_eq!(n.url, "https://console/build/42");
    }

    #[test]
    fn error_code_is_the_reason_when_there_is_no_summary() {
        let inner = r#"{"source":"aws.codepipeline","account":"123456789012",
            "detail":{"pipeline":"taller-pipeline","stage":"Desplegar","state":"FAILED",
                "execution-result":{"error-code":"JobFailed"}}}"#;
        let body = serde_json::json!({"Type":"Notification","Message": inner}).to_string();
        let SnsMessage::Event(n) = parse_sns_message(&body).unwrap() else {
            panic!("expected event");
        };
        assert_eq!(n.reason, "JobFailed");
    }

    #[test]
    fn codebuild_event_keeps_the_build_id_tail_and_log_link() {
        let inner = r#"{"source":"aws.codebuild","account":"123456789012",
            "region":"us-east-2",
            "detail":{"project-name":"taller-build","build-status":"FAILED",
                "current-phase":"BUILD",
                "build-id":"arn:aws:codebuild:us-east-2:123456789012:build/taller-build:9f8e",
                "additional-information":{"logs":{"deep-link":"https://logs/x"}}}}"#;
        let body = serde_json::json!({"Type":"Notification","Message": inner}).to_string();
        let SnsMessage::Event(n) = parse_sns_message(&body).unwrap() else {
            panic!("expected event");
        };
        assert_eq!(n.phase, "BUILD");
        assert_eq!(n.execution, "taller-build:9f8e");
        assert_eq!(n.url, "https://logs/x");
    }

    #[test]
    fn pipeline_execution_link_is_built_from_region_and_execution() {
        let inner = r#"{"source":"aws.codepipeline","account":"123456789012",
            "region":"us-east-2",
            "detail":{"pipeline":"taller-pipeline","state":"SUCCEEDED",
                "execution-id":"e1a2b3c4"}}"#;
        let body = serde_json::json!({"Type":"Notification","Message": inner}).to_string();
        let SnsMessage::Event(n) = parse_sns_message(&body).unwrap() else {
            panic!("expected event");
        };
        assert_eq!(
            n.url,
            "https://us-east-2.console.aws.amazon.com/codesuite/codepipeline/pipelines\
             /taller-pipeline/executions/e1a2b3c4?region=us-east-2"
        );
    }

    #[test]
    fn region_falls_back_to_the_first_resource_arn() {
        let inner = r#"{"source":"aws.codepipeline","account":"123456789012",
            "resources":["arn:aws:codepipeline:eu-west-1:123456789012:taller-pipeline"],
            "detail":{"pipeline":"taller-pipeline","state":"STARTED"}}"#;
        let body = serde_json::json!({"Type":"Notification","Message": inner}).to_string();
        let SnsMessage::Event(n) = parse_sns_message(&body).unwrap() else {
            panic!("expected event");
        };
        assert_eq!(n.region, "eu-west-1");
    }

    #[test]
    fn no_region_means_no_link() {
        let inner = r#"{"source":"aws.codepipeline","account":"123456789012",
            "detail":{"pipeline":"taller-pipeline","state":"STARTED"}}"#;
        let body = serde_json::json!({"Type":"Notification","Message": inner}).to_string();
        let SnsMessage::Event(n) = parse_sns_message(&body).unwrap() else {
            panic!("expected event");
        };
        assert_eq!(n.url, "");
    }

    #[test]
    fn eventbridge_source_wins_over_alarm_shape() {
        let inner = r#"{"source":"aws.codepipeline","account":"123456789012",
            "AlarmName":"no-usar","AWSAccountId":"999999999999",
            "detail":{"pipeline":"taller-pipeline","state":"STARTED"}}"#;
        let body = serde_json::json!({"Type":"Notification","Message": inner}).to_string();
        let SnsMessage::Event(n) = parse_sns_message(&body).unwrap() else {
            panic!("expected event");
        };
        assert_eq!(n.pod, "123456789012");
        assert_eq!(n.source, "aws.codepipeline");
        assert_eq!(n.detail, "taller-pipeline");
    }

    #[test]
    fn json_without_known_fields_stays_neutral() {
        let body =
            serde_json::json!({"Type":"Notification","Message":"{\"hola\":\"mundo\"}"}).to_string();
        let SnsMessage::Event(n) = parse_sns_message(&body).unwrap() else {
            panic!("expected event");
        };
        assert_eq!(n.pod, "desconocido");
        assert_eq!(n.source, "evento");
        assert_eq!(n.state, "");
        assert_eq!(n.detail, "");
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
