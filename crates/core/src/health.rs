//! Pure domain for the three-tier health check.
//!
//! The workshop teaches liveness, readiness, and startup as three separate
//! questions with three different consequences. This module holds the decision
//! rules and the response body; probing, timing, and env access live in the
//! server shell.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A dependency the background prober watches, and that the workshop can break
/// on purpose. The names are the wire values used by `/health/simulate`, and by
/// the `health-fault` event payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dependency {
    /// DynamoDB. Hard: without it the scenario console has no answer to give.
    Dynamo,
    /// The rendered content. Soft: a broken build degrades the site, and the
    /// instance keeps serving every page that did render.
    Content,
}

impl Dependency {
    /// Every dependency, in report order.
    pub const ALL: [Self; 2] = [Self::Dynamo, Self::Content];

    /// The wire name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dynamo => "dynamodb",
            Self::Content => "content",
        }
    }

    /// Parses a wire name. Case, and surrounding space, are ignored.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "dynamodb" | "dynamo" => Some(Self::Dynamo),
            "content" => Some(Self::Content),
            _ => None,
        }
    }

    /// Whether a failure of this dependency takes the instance out of rotation.
    #[must_use]
    pub fn criticality(self) -> Criticality {
        match self {
            Self::Dynamo => Criticality::Hard,
            Self::Content => Criticality::Soft,
        }
    }
}

/// How a dependency failure affects the ability to serve traffic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Criticality {
    /// No useful answer without it: readiness fails.
    Hard,
    /// Degraded but serviceable: readiness still passes.
    Soft,
}

/// Result of one dependency probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeStatus {
    Ok,
    Fail,
}

/// One probed dependency, as reported in the response body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DependencyReport {
    pub name: String,
    pub criticality: Criticality,
    pub status: ProbeStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl DependencyReport {
    /// A passing probe.
    #[must_use]
    pub fn ok(name: impl Into<String>, criticality: Criticality, latency_ms: u64) -> Self {
        Self {
            name: name.into(),
            criticality,
            status: ProbeStatus::Ok,
            latency_ms: Some(latency_ms),
            error: None,
        }
    }

    /// A failing probe, carrying the reason.
    #[must_use]
    pub fn fail(
        name: impl Into<String>,
        criticality: Criticality,
        error: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            criticality,
            status: ProbeStatus::Fail,
            latency_ms: None,
            error: Some(error.into()),
        }
    }

    /// Whether this report blocks readiness.
    #[must_use]
    pub fn blocks_readiness(&self) -> bool {
        self.criticality == Criticality::Hard && self.status == ProbeStatus::Fail
    }
}

/// Where the process is in its own lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Lifecycle {
    /// Still warming up: the first probe has not completed.
    Starting,
    /// Serving traffic.
    Ready,
    /// Shutdown began; readiness reports `503` so the balancer deregisters
    /// this target before the process stops accepting connections.
    Draining,
}

/// Aggregate reported in the body. Soft failures change this without changing
/// the HTTP status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Overall {
    Ok,
    Degraded,
    Unavailable,
}

/// Which of the three endpoints produced a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Check {
    Live,
    Ready,
    Startup,
}

/// The JSON body every health endpoint returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HealthBody {
    pub check: Check,
    pub status: Overall,
    pub lifecycle: Lifecycle,
    pub version: String,
    /// Milliseconds since the background prober last finished a round.
    pub last_probe_ms: u64,
    pub checks: Vec<DependencyReport>,
}

/// Everything the pure rules need to answer one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthView {
    pub lifecycle: Lifecycle,
    pub dependencies: Vec<DependencyReport>,
    /// Milliseconds since the prober last finished a round.
    pub last_probe_ms: u64,
    /// Configured probe interval, in milliseconds.
    pub interval_ms: u64,
    pub version: String,
}

/// Tunables for the health subsystem, parsed once at the process boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthSettings {
    /// The feature flag. Everything else is inert while this is `false`.
    pub enabled: bool,
    /// How often the background prober runs.
    pub interval_secs: u64,
    /// Per-dependency probe timeout.
    pub timeout_ms: u64,
    /// How long readiness reports `503` before connections stop being accepted.
    pub drain_secs: u64,
}

impl Default for HealthSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_secs: DEFAULT_INTERVAL_SECS,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            drain_secs: DEFAULT_DRAIN_SECS,
        }
    }
}

/// Raw environment values, before parsing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HealthEnv<'a> {
    pub enabled: Option<&'a str>,
    pub interval_secs: Option<&'a str>,
    pub timeout_ms: Option<&'a str>,
    pub drain_secs: Option<&'a str>,
}

const DEFAULT_INTERVAL_SECS: u64 = 5;
const DEFAULT_TIMEOUT_MS: u64 = 2_000;
const DEFAULT_DRAIN_SECS: u64 = 15;

/// How many probe rounds may be missed before liveness reports a stuck process.
const STALE_ROUNDS: u64 = 4;

/// How long a dependency stays broken when the payload omits `seconds`.
pub const DEFAULT_FAULT_SECONDS: u64 = 60;

/// Hard cap on an injected outage, so a mistyped value cannot park the instance
/// out of rotation for the rest of the session.
pub const MAX_FAULT_SECONDS: u64 = 600;

/// A requested outage: which dependency to break, and for how long.
///
/// `seconds` of zero is meaningful: it restores the dependency immediately,
/// which is what the widget's restore button sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthFaultConfig {
    pub dependency: Dependency,
    pub seconds: u64,
}

/// Unvalidated deserialization target for [`HealthFaultConfig::parse`].
#[derive(Deserialize)]
struct HealthFaultRaw {
    #[serde(default)]
    dependency: Option<String>,
    #[serde(default)]
    seconds: Option<u64>,
}

impl HealthFaultConfig {
    /// Parses a `health-fault` payload. An absent dependency means the hard one
    /// (the interesting case); an absent duration means [`DEFAULT_FAULT_SECONDS`].
    pub fn parse(payload: &str) -> Result<Self, crate::Error> {
        let raw: HealthFaultRaw = serde_json::from_str(payload)
            .map_err(|e| crate::Error::MalformedEvent(e.to_string()))?;
        let dependency = match raw.dependency.as_deref() {
            None => Dependency::Dynamo,
            Some(name) => Dependency::parse(name).ok_or_else(|| {
                crate::Error::MalformedEvent(format!("unknown health dependency {name:?}"))
            })?,
        };
        Ok(Self {
            dependency,
            seconds: raw
                .seconds
                .unwrap_or(DEFAULT_FAULT_SECONDS)
                .min(MAX_FAULT_SECONDS),
        })
    }
}

// ---------------------------------------------------------------------------
// Pure rules
// ---------------------------------------------------------------------------

/// Parses the health settings, falling back to defaults on absent or malformed
/// values. A bad number never aborts startup: the workshop values are a
/// teaching knob, not a correctness boundary.
#[must_use]
pub fn parse_health_settings(env: HealthEnv<'_>) -> HealthSettings {
    HealthSettings {
        enabled: parse_flag(env.enabled),
        interval_secs: parse_positive(env.interval_secs, DEFAULT_INTERVAL_SECS),
        timeout_ms: parse_positive(env.timeout_ms, DEFAULT_TIMEOUT_MS),
        // Zero is meaningful here: it turns draining off.
        drain_secs: parse_number(env.drain_secs, DEFAULT_DRAIN_SECS),
    }
}

/// Reads a boolean environment flag. Absent, empty, or unrecognised is `false`.
#[must_use]
pub fn parse_flag(raw: Option<&str>) -> bool {
    matches!(
        raw.map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref()
            .unwrap_or(""),
        "1" | "true" | "yes" | "on"
    )
}

fn parse_number(raw: Option<&str>, fallback: u64) -> u64 {
    match raw.map(str::trim) {
        Some(value) if !value.is_empty() => value.parse::<u64>().unwrap_or(fallback),
        _ => fallback,
    }
}

fn parse_positive(raw: Option<&str>, fallback: u64) -> u64 {
    match parse_number(raw, fallback) {
        0 => fallback,
        n => n,
    }
}

/// Whether the background prober stopped making progress.
///
/// This is the liveness signal that a bare `200` cannot give: the request path
/// answering proves the listener works, but a prober that stopped ticking means
/// the runtime is stuck.
#[must_use]
pub fn prober_is_stale(last_probe_ms: u64, interval_ms: u64) -> bool {
    let budget = interval_ms.saturating_mul(STALE_ROUNDS);
    budget > 0 && last_probe_ms > budget
}

/// Aggregate status: any failing hard dependency is `Unavailable`, any failing
/// soft dependency is `Degraded`.
#[must_use]
pub fn overall(lifecycle: Lifecycle, dependencies: &[DependencyReport]) -> Overall {
    if dependencies.iter().any(DependencyReport::blocks_readiness) {
        return Overall::Unavailable;
    }
    let soft_failed = dependencies
        .iter()
        .any(|d| d.status == ProbeStatus::Fail && d.criticality == Criticality::Soft);
    match (lifecycle, soft_failed) {
        (Lifecycle::Starting | Lifecycle::Draining, _) | (Lifecycle::Ready, true) => {
            Overall::Degraded
        }
        (Lifecycle::Ready, false) => Overall::Ok,
    }
}

/// Liveness: is the process alive, and not stuck? Failing means the container
/// is replaced, so nothing but a stuck runtime may fail it.
#[must_use]
pub fn live_code(view: &HealthView) -> u16 {
    if prober_is_stale(view.last_probe_ms, view.interval_ms) {
        503
    } else {
        200
    }
}

/// Readiness: can this instance serve traffic right now? Failing means the
/// balancer stops sending requests; the task keeps running.
#[must_use]
pub fn ready_code(view: &HealthView) -> u16 {
    match view.lifecycle {
        Lifecycle::Starting | Lifecycle::Draining => 503,
        Lifecycle::Ready => {
            if view
                .dependencies
                .iter()
                .any(DependencyReport::blocks_readiness)
            {
                503
            } else {
                200
            }
        }
    }
}

/// Startup: has the process finished warming up? Failing means "keep waiting",
/// never "replace me".
#[must_use]
pub fn startup_code(view: &HealthView) -> u16 {
    match view.lifecycle {
        Lifecycle::Starting => 503,
        Lifecycle::Ready | Lifecycle::Draining => 200,
    }
}

/// The HTTP status for one endpoint.
#[must_use]
pub fn status_code(check: Check, view: &HealthView) -> u16 {
    match check {
        Check::Live => live_code(view),
        Check::Ready => ready_code(view),
        Check::Startup => startup_code(view),
    }
}

/// Builds the response body for one endpoint.
#[must_use]
pub fn body(check: Check, view: &HealthView) -> HealthBody {
    HealthBody {
        check,
        status: overall(view.lifecycle, &view.dependencies),
        lifecycle: view.lifecycle,
        version: view.version.clone(),
        last_probe_ms: view.last_probe_ms,
        checks: view.dependencies.clone(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::{
        Check, Criticality, DEFAULT_FAULT_SECONDS, Dependency, DependencyReport, HealthEnv,
        HealthFaultConfig, HealthView, Lifecycle, MAX_FAULT_SECONDS, Overall, ProbeStatus, body,
        live_code, overall, parse_flag, parse_health_settings, prober_is_stale, ready_code,
        startup_code, status_code,
    };

    fn view(lifecycle: Lifecycle, dependencies: Vec<DependencyReport>) -> HealthView {
        HealthView {
            lifecycle,
            dependencies,
            last_probe_ms: 0,
            interval_ms: 5_000,
            version: "1.2.3".to_owned(),
        }
    }

    fn hard_fail() -> DependencyReport {
        DependencyReport::fail("dynamodb", Criticality::Hard, "timeout")
    }

    fn soft_fail() -> DependencyReport {
        DependencyReport::fail("content", Criticality::Soft, "broken")
    }

    fn hard_ok() -> DependencyReport {
        DependencyReport::ok("dynamodb", Criticality::Hard, 3)
    }

    #[test]
    fn report_constructors_set_status_and_fields() {
        let ok = hard_ok();
        assert_eq!(ok.status, ProbeStatus::Ok);
        assert_eq!(ok.latency_ms, Some(3));
        assert_eq!(ok.error, None);

        let bad = hard_fail();
        assert_eq!(bad.status, ProbeStatus::Fail);
        assert_eq!(bad.latency_ms, None);
        assert_eq!(bad.error.as_deref(), Some("timeout"));
    }

    #[test]
    fn only_failing_hard_dependencies_block_readiness() {
        assert!(hard_fail().blocks_readiness());
        assert!(!soft_fail().blocks_readiness());
        assert!(!hard_ok().blocks_readiness());
    }

    #[test]
    fn parse_flag_accepts_the_usual_truthy_spellings() {
        for raw in ["1", "true", "TRUE", " yes ", "on"] {
            assert!(parse_flag(Some(raw)), "{raw:?} should be truthy");
        }
        for raw in ["0", "false", "", "  ", "maybe"] {
            assert!(!parse_flag(Some(raw)), "{raw:?} should be falsy");
        }
        assert!(!parse_flag(None));
    }

    #[test]
    fn settings_fall_back_to_defaults() {
        let settings = parse_health_settings(HealthEnv::default());
        assert!(!settings.enabled);
        assert_eq!(settings.interval_secs, 5);
        assert_eq!(settings.timeout_ms, 2_000);
        assert_eq!(settings.drain_secs, 15);
    }

    #[test]
    fn settings_parse_valid_values() {
        let settings = parse_health_settings(HealthEnv {
            enabled: Some("true"),
            interval_secs: Some("10"),
            timeout_ms: Some("500"),
            drain_secs: Some("30"),
        });
        assert!(settings.enabled);
        assert_eq!(settings.interval_secs, 10);
        assert_eq!(settings.timeout_ms, 500);
        assert_eq!(settings.drain_secs, 30);
    }

    #[test]
    fn settings_reject_junk_and_zero_intervals() {
        let settings = parse_health_settings(HealthEnv {
            enabled: Some("1"),
            interval_secs: Some("abc"),
            timeout_ms: Some("0"),
            drain_secs: Some("nope"),
        });
        assert_eq!(settings.interval_secs, 5);
        assert_eq!(settings.timeout_ms, 2_000);
        assert_eq!(settings.drain_secs, 15);
    }

    #[test]
    fn drain_may_be_disabled_with_zero() {
        let settings = parse_health_settings(HealthEnv {
            drain_secs: Some("0"),
            ..HealthEnv::default()
        });
        assert_eq!(settings.drain_secs, 0);
    }

    #[test]
    fn prober_is_stale_after_four_missed_rounds() {
        assert!(!prober_is_stale(0, 5_000));
        assert!(!prober_is_stale(20_000, 5_000));
        assert!(prober_is_stale(20_001, 5_000));
    }

    #[test]
    fn overall_ranks_hard_failures_above_soft_ones() {
        assert_eq!(overall(Lifecycle::Ready, &[hard_ok()]), Overall::Ok);
        assert_eq!(
            overall(Lifecycle::Ready, &[hard_ok(), soft_fail()]),
            Overall::Degraded
        );
        assert_eq!(
            overall(Lifecycle::Ready, &[hard_fail(), soft_fail()]),
            Overall::Unavailable
        );
    }

    #[test]
    fn overall_reports_degraded_while_starting_or_draining() {
        assert_eq!(overall(Lifecycle::Starting, &[]), Overall::Degraded);
        assert_eq!(overall(Lifecycle::Draining, &[]), Overall::Degraded);
        // A hard failure still wins over the lifecycle.
        assert_eq!(
            overall(Lifecycle::Draining, &[hard_fail()]),
            Overall::Unavailable
        );
    }

    #[test]
    fn readiness_fails_while_starting_and_draining() {
        assert_eq!(ready_code(&view(Lifecycle::Starting, vec![hard_ok()])), 503);
        assert_eq!(ready_code(&view(Lifecycle::Draining, vec![hard_ok()])), 503);
        assert_eq!(ready_code(&view(Lifecycle::Ready, vec![hard_ok()])), 200);
    }

    #[test]
    fn readiness_follows_hard_dependencies_only() {
        assert_eq!(ready_code(&view(Lifecycle::Ready, vec![hard_fail()])), 503);
        assert_eq!(ready_code(&view(Lifecycle::Ready, vec![soft_fail()])), 200);
    }

    #[test]
    fn startup_passes_once_the_first_probe_lands() {
        assert_eq!(startup_code(&view(Lifecycle::Starting, vec![])), 503);
        assert_eq!(startup_code(&view(Lifecycle::Ready, vec![])), 200);
        assert_eq!(startup_code(&view(Lifecycle::Draining, vec![])), 200);
    }

    #[test]
    fn liveness_ignores_dependencies_and_lifecycle() {
        assert_eq!(
            live_code(&view(Lifecycle::Starting, vec![hard_fail()])),
            200
        );
        assert_eq!(
            live_code(&view(Lifecycle::Draining, vec![hard_fail()])),
            200
        );
    }

    #[test]
    fn liveness_fails_only_when_the_prober_stalls() {
        let mut stuck = view(Lifecycle::Ready, vec![hard_ok()]);
        stuck.last_probe_ms = 60_000;
        assert_eq!(live_code(&stuck), 503);
    }

    #[test]
    fn status_code_dispatches_per_check() {
        let starting = view(Lifecycle::Starting, vec![hard_ok()]);
        assert_eq!(status_code(Check::Live, &starting), 200);
        assert_eq!(status_code(Check::Ready, &starting), 503);
        assert_eq!(status_code(Check::Startup, &starting), 503);
    }

    #[test]
    fn body_carries_the_snapshot() {
        let view = view(Lifecycle::Ready, vec![hard_ok(), soft_fail()]);
        let body = body(Check::Ready, &view);
        assert_eq!(body.check, Check::Ready);
        assert_eq!(body.status, Overall::Degraded);
        assert_eq!(body.lifecycle, Lifecycle::Ready);
        assert_eq!(body.version, "1.2.3");
        assert_eq!(body.checks.len(), 2);
    }

    #[test]
    fn dependency_names_round_trip() {
        for dependency in Dependency::ALL {
            assert_eq!(Dependency::parse(dependency.as_str()), Some(dependency));
        }
        assert_eq!(Dependency::parse(" DYNAMO "), Some(Dependency::Dynamo));
        assert_eq!(Dependency::parse("redis"), None);
        assert_eq!(Dependency::parse(""), None);
    }

    #[test]
    fn dependency_criticality_matches_the_probe_reports() {
        assert_eq!(Dependency::Dynamo.criticality(), Criticality::Hard);
        assert_eq!(Dependency::Content.criticality(), Criticality::Soft);
    }

    #[test]
    fn fault_payload_defaults_to_the_hard_dependency_for_a_minute() {
        let cfg = HealthFaultConfig::parse("{}").unwrap();
        assert_eq!(cfg.dependency, Dependency::Dynamo);
        assert_eq!(cfg.seconds, DEFAULT_FAULT_SECONDS);
    }

    #[test]
    fn fault_payload_reads_both_fields() {
        let cfg = HealthFaultConfig::parse(r#"{"dependency":"content","seconds":30}"#).unwrap();
        assert_eq!(cfg.dependency, Dependency::Content);
        assert_eq!(cfg.seconds, 30);
    }

    #[test]
    fn fault_seconds_are_capped_and_zero_is_kept() {
        let long = HealthFaultConfig::parse(r#"{"seconds":99999}"#).unwrap();
        assert_eq!(long.seconds, MAX_FAULT_SECONDS);
        // Zero means "restore now"; it must survive parsing.
        let zero = HealthFaultConfig::parse(r#"{"seconds":0}"#).unwrap();
        assert_eq!(zero.seconds, 0);
    }

    #[test]
    fn fault_payload_rejects_unknown_dependencies_and_junk() {
        assert!(HealthFaultConfig::parse(r#"{"dependency":"redis"}"#).is_err());
        assert!(HealthFaultConfig::parse("not json").is_err());
    }

    #[test]
    fn body_serializes_with_lowercase_tags() {
        let json = serde_json::to_string(&body(
            Check::Ready,
            &view(Lifecycle::Ready, vec![soft_fail()]),
        ))
        .unwrap();
        assert!(json.contains(r#""check":"ready""#), "{json}");
        assert!(json.contains(r#""status":"degraded""#), "{json}");
        assert!(json.contains(r#""criticality":"soft""#), "{json}");
        // `latency_ms` is absent on a failure, not null.
        assert!(!json.contains("latency_ms"), "{json}");
    }
}
