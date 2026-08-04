//! Imperative shell for the three-tier health check.
//!
//! Off by default: everything here is inert unless `CB_HEALTH_CHECKS` is set.
//! The subsystem exists to make the guide's health-check section observable, so
//! the workshop can watch a target leave rotation, and come back, without
//! touching the deployed application.
//!
//! Shape: a background prober owns every dependency call and writes a snapshot;
//! the three handlers only read that snapshot, so a health request never issues
//! I/O of its own.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Json;
use axum::Router;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use courses_apps::{AppsCtx, HealthFaults};
use courses_core::{
    Check, Dependency, DependencyReport, HealthEnv, HealthSettings, HealthView, Lifecycle,
    health_body, parse_health_settings, status_code, token_matches,
};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::site::{SiteHandle, SiteState};

/// Feature flag. Absent, or falsy, leaves the whole subsystem off.
pub const HEALTH_CHECKS_ENV: &str = "CB_HEALTH_CHECKS";

/// Seconds between probe rounds.
const INTERVAL_ENV: &str = "CB_HEALTH_INTERVAL_SECS";

/// Per-dependency probe timeout, in milliseconds.
const TIMEOUT_ENV: &str = "CB_HEALTH_TIMEOUT_MS";

/// Seconds spent draining after the shutdown signal, before connections close.
const DRAIN_ENV: &str = "CB_HEALTH_DRAIN_SECS";

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

/// Reads the health settings from the environment. Malformed values fall back
/// to their defaults; see `courses_core::parse_health_settings`.
#[must_use]
pub fn settings_from_env() -> HealthSettings {
    let enabled = std::env::var(HEALTH_CHECKS_ENV).ok();
    let interval = std::env::var(INTERVAL_ENV).ok();
    let timeout = std::env::var(TIMEOUT_ENV).ok();
    let drain = std::env::var(DRAIN_ENV).ok();
    parse_health_settings(HealthEnv {
        enabled: enabled.as_deref(),
        interval_secs: interval.as_deref(),
        timeout_ms: timeout.as_deref(),
        drain_secs: drain.as_deref(),
    })
}

/// Snapshot written by the prober, read by the handlers.
struct HealthState {
    lifecycle: Lifecycle,
    dependencies: Vec<DependencyReport>,
    last_probe: Instant,
}

/// Cheap, cloneable handle over the health snapshot.
#[derive(Clone)]
pub struct HealthHandle {
    state: Arc<RwLock<HealthState>>,
    settings: HealthSettings,
}

impl HealthHandle {
    #[must_use]
    pub fn new(settings: HealthSettings) -> Self {
        Self {
            state: Arc::new(RwLock::new(HealthState {
                lifecycle: Lifecycle::Starting,
                dependencies: Vec::new(),
                last_probe: Instant::now(),
            })),
            settings,
        }
    }

    #[must_use]
    pub fn settings(&self) -> HealthSettings {
        self.settings
    }

    /// Reads the snapshot into the pure view the decision rules consume.
    async fn view(&self) -> HealthView {
        let state = self.state.read().await;
        HealthView {
            lifecycle: state.lifecycle,
            dependencies: state.dependencies.clone(),
            last_probe_ms: u64::try_from(state.last_probe.elapsed().as_millis())
                .unwrap_or(u64::MAX),
            interval_ms: self.settings.interval_secs.saturating_mul(1_000),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }

    /// Stores a finished probe round. The first round ends the startup window.
    async fn record(&self, dependencies: Vec<DependencyReport>) {
        let mut state = self.state.write().await;
        state.dependencies = dependencies;
        state.last_probe = Instant::now();
        if state.lifecycle == Lifecycle::Starting {
            state.lifecycle = Lifecycle::Ready;
            tracing::info!("health: startup finished; readiness now reports 200");
        }
    }

    /// Flips the process into draining. Readiness answers `503` from here on,
    /// which is what lets the balancer deregister before connections close.
    pub async fn begin_draining(&self) {
        let mut state = self.state.write().await;
        state.lifecycle = Lifecycle::Draining;
        tracing::info!("health: draining; readiness now reports 503");
    }
}

// ---------------------------------------------------------------------------
// Background prober
// ---------------------------------------------------------------------------

/// Everything one probe round needs.
#[derive(Clone)]
pub struct ProbeCtx {
    pub handle: HealthHandle,
    pub apps: AppsCtx,
    pub site: SiteHandle,
}

/// Runs dependency probes forever, on the configured interval.
///
/// The returned task is detached on purpose: it lives as long as the process,
/// and its liveness is itself reported through `/health/live`.
pub fn spawn_prober(ctx: ProbeCtx) {
    let interval = Duration::from_secs(ctx.handle.settings.interval_secs);
    let timeout = Duration::from_millis(ctx.handle.settings.timeout_ms);
    tokio::spawn(async move {
        loop {
            let dependencies = vec![
                probe_dynamo(&ctx.apps, timeout).await,
                probe_content(&ctx.site, &ctx.apps.faults).await,
            ];
            ctx.handle.record(dependencies).await;
            tokio::time::sleep(interval).await;
        }
    });
}

/// The report an injected outage produces, in place of the real probe.
///
/// The reason is Spanish on purpose: unlike the SDK error strings beside it,
/// this text is written for the participant reading the widget.
fn injected(dependency: Dependency) -> DependencyReport {
    DependencyReport::fail(
        dependency.as_str(),
        dependency.criticality(),
        "falla inyectada desde la guía",
    )
}

/// Probes DynamoDB with `DescribeTable`, under a timeout.
async fn probe_dynamo(apps: &AppsCtx, timeout: Duration) -> DependencyReport {
    let dependency = Dependency::Dynamo;
    if apps.faults.is_active(dependency).await {
        return injected(dependency);
    }
    let started = Instant::now();
    let call = apps.dynamo.describe_table().table_name(&apps.table).send();
    match tokio::time::timeout(timeout, call).await {
        Ok(Ok(_)) => DependencyReport::ok(
            dependency.as_str(),
            dependency.criticality(),
            u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        ),
        Ok(Err(e)) => DependencyReport::fail(
            dependency.as_str(),
            dependency.criticality(),
            service_error(&e),
        ),
        Err(_) => DependencyReport::fail(
            dependency.as_str(),
            dependency.criticality(),
            format!("timeout after {}ms", timeout.as_millis()),
        ),
    }
}

/// Reports the rendered site. Broken content degrades the answer without
/// taking the instance out of rotation.
async fn probe_content(site: &SiteHandle, faults: &HealthFaults) -> DependencyReport {
    let dependency = Dependency::Content;
    if faults.is_active(dependency).await {
        return injected(dependency);
    }
    match site.load().as_ref() {
        SiteState::Ready(_) => {
            DependencyReport::ok(dependency.as_str(), dependency.criticality(), 0)
        }
        SiteState::Broken(message) => DependencyReport::fail(
            dependency.as_str(),
            dependency.criticality(),
            message.clone(),
        ),
    }
}

/// Flattens an AWS SDK error into one line for the response body.
fn service_error(error: &(impl std::error::Error + ?Sized)) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(inner) = source {
        message.push_str(": ");
        message.push_str(&inner.to_string());
        source = inner.source();
    }
    message
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

/// Handler state: the snapshot, the fault registry, and the secret guarding
/// fault injection.
#[derive(Clone)]
struct HealthRoutes {
    handle: HealthHandle,
    faults: HealthFaults,
    secret: Option<Arc<String>>,
}

/// The query on `POST /health/simulate`.
#[derive(Deserialize)]
struct SimulateQuery {
    dependency: String,
    #[serde(default)]
    fail: Option<String>,
    /// Optional outage length. Absent means "until cleared".
    #[serde(default)]
    seconds: Option<u64>,
    #[serde(default)]
    secret: Option<String>,
}

/// Builds the health router. Callers merge this only when the flag is on.
pub fn router(handle: HealthHandle, faults: HealthFaults, secret: Option<Arc<String>>) -> Router {
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .route("/health/startup", get(startup))
        .route("/health/simulate", post(simulate))
        .with_state(HealthRoutes {
            handle,
            faults,
            secret,
        })
}

async fn live(State(state): State<HealthRoutes>) -> Response {
    answer(Check::Live, &state.handle).await
}

async fn ready(State(state): State<HealthRoutes>) -> Response {
    answer(Check::Ready, &state.handle).await
}

async fn startup(State(state): State<HealthRoutes>) -> Response {
    answer(Check::Startup, &state.handle).await
}

/// Reads the snapshot, and lets the pure rules decide the code and the body.
async fn answer(check: Check, handle: &HealthHandle) -> Response {
    let view = handle.view().await;
    let code = StatusCode::from_u16(status_code(check, &view))
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (code, Json(health_body(check, &view))).into_response()
}

/// Forces a dependency to report failure, so the effect on each endpoint is
/// observable during the workshop. Guarded by the apps secret when one is set.
///
/// This is the terminal-side twin of the guide's `<cb-health>` widget: without
/// `seconds` the outage lasts until it is cleared, which the widget never does.
async fn simulate(
    State(state): State<HealthRoutes>,
    Query(query): Query<SimulateQuery>,
) -> Response {
    if let Some(expected) = state.secret.as_deref() {
        let provided = query.secret.as_deref().unwrap_or_default();
        if !token_matches(expected, provided) {
            tracing::warn!("rejected /health/simulate: bad or missing secret");
            return StatusCode::UNAUTHORIZED.into_response();
        }
    }
    let Some(dependency) = Dependency::parse(&query.dependency) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "unknown dependency",
                "known": Dependency::ALL.map(Dependency::as_str),
            })),
        )
            .into_response();
    };
    // Absent `fail` means "start failing"; the clear path is explicit.
    let fail = query
        .fail
        .as_deref()
        .is_none_or(|raw| courses_core::parse_flag(Some(raw)));
    if fail {
        let duration = query.seconds.map(Duration::from_secs);
        state.faults.inject(dependency, duration).await;
    } else {
        state.faults.clear(dependency).await;
    }
    tracing::warn!(
        dependency = dependency.as_str(),
        fail,
        seconds = query.seconds,
        "health: forced dependency state changed"
    );
    Json(serde_json::json!({ "forced": active_faults(&state.faults).await })).into_response()
}

/// The dependencies currently forced to fail, by wire name.
async fn active_faults(faults: &HealthFaults) -> Vec<&'static str> {
    let mut active = Vec::new();
    for dependency in Dependency::ALL {
        if faults.is_active(dependency).await {
            active.push(dependency.as_str());
        }
    }
    active
}
