use std::convert::Infallible;
use std::sync::{Arc, Mutex};

use axum::Json;
use axum::Router;
use axum::extract::{FromRef, Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use courses_apps::{AppsCtx, Outcome};
use courses_core::{
    Event, EventId, Notification, RecentIds, RenderedSite, Seen, SnsMessage, parse_event,
    parse_gate, parse_sns_message, token_matches,
};
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

const APPS_JS: &str = include_str!("../static/apps.js");

/// Buffer of in-flight events per subscriber before a slow client lags.
const NOTIFICATION_CHANNEL_CAPACITY: usize = 256;

/// Dedup window for the `POST /events` idempotency check.
const RECENT_IDS_CAPACITY: usize = 1024;

/// Default DynamoDB table name when `CB_APPS_TABLE` is unset.
const DEFAULT_APPS_TABLE: &str = "courses-apps";

/// Default public collections when `CB_APPS_PUBLIC_COLLECTIONS` is unset.
const DEFAULT_PUBLIC_COLLECTIONS: &str = "counters";

/// Environment variable holding the SNS webhook secret.
const HOOK_TOKEN_ENV: &str = "CB_HOOK_TOKEN";

/// Environment variable holding the apps-endpoint secret.
const APPS_SECRET_ENV: &str = "CB_APPS_SECRET";

/// Environment variable controlling which handler kinds require the apps secret.
/// Values: `"all"` (or absent when secret is set) → gate everything; otherwise
/// a comma-separated list of kind names (`cpu-burst`, `counter`).
const APPS_GATED_ENV: &str = "CB_APPS_GATED";

/// Environment variable holding the DynamoDB table name.
const APPS_TABLE_ENV: &str = "CB_APPS_TABLE";

/// Environment variable holding a comma-separated whitelist of readable collections.
const APPS_PUBLIC_COLLECTIONS_ENV: &str = "CB_APPS_PUBLIC_COLLECTIONS";

/// Shared handler state.
///
/// `FromRef` lets page handlers extract `State<Arc<RenderedSite>>` and SSE
/// handlers extract `State<broadcast::Sender<Event>>` without receiving the
/// full `AppState`.
#[derive(Clone)]
pub struct AppState {
    site: Arc<RenderedSite>,
    hook_token: Option<Arc<String>>,
    apps: AppsCtx,
    recent: Arc<Mutex<RecentIds>>,
}

/// The `?token=` query on the SNS ingestion endpoint.
#[derive(Deserialize)]
struct HookAuth {
    token: Option<String>,
}

/// The `?secret=` query on the `POST /events` endpoint.
#[derive(Deserialize)]
struct AppSecret {
    secret: Option<String>,
}

impl FromRef<AppState> for Arc<RenderedSite> {
    fn from_ref(state: &AppState) -> Self {
        Arc::clone(&state.site)
    }
}

impl FromRef<AppState> for broadcast::Sender<Event> {
    fn from_ref(state: &AppState) -> Self {
        state.apps.bus.clone()
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Builds the application router. Async because it loads AWS SDK config and
/// reads environment variables.
pub async fn router(site: Arc<RenderedSite>) -> Router {
    // --- SNS hook token ---
    let hook_token = std::env::var(HOOK_TOKEN_ENV).ok().map(Arc::new);
    if hook_token.is_none() {
        tracing::warn!(
            "{HOOK_TOKEN_ENV} is unset: /hooks/notifications is open (no token required)"
        );
    }

    // --- Apps gate config ---
    let apps_secret = std::env::var(APPS_SECRET_ENV).ok();
    if apps_secret.is_none() {
        tracing::warn!("{APPS_SECRET_ENV} is unset: /events endpoint is open (no secret required)");
    }
    let apps_gated = std::env::var(APPS_GATED_ENV).ok();
    let gate_config = parse_gate(apps_secret.as_deref(), apps_gated.as_deref());
    for unknown in &gate_config.unknown_kinds {
        tracing::warn!("unknown handler kind in {APPS_GATED_ENV}: {unknown:?} — skipped");
    }
    let gate = gate_config.gate;

    // --- DynamoDB table ---
    let table = std::env::var(APPS_TABLE_ENV).unwrap_or_else(|_| {
        tracing::info!(
            "{APPS_TABLE_ENV} is unset: using default table name {DEFAULT_APPS_TABLE:?}"
        );
        DEFAULT_APPS_TABLE.to_string()
    });

    // --- Public collections whitelist ---
    let public_raw = std::env::var(APPS_PUBLIC_COLLECTIONS_ENV)
        .unwrap_or_else(|_| DEFAULT_PUBLIC_COLLECTIONS.to_string());
    let public_collections: Vec<String> = public_raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // --- AWS SDK ---
    tracing::info!("resolving AWS configuration...");
    let aws_cfg = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let dynamo = aws_sdk_dynamodb::Client::new(&aws_cfg);
    let cloudwatch = aws_sdk_cloudwatch::Client::new(&aws_cfg);
    let region = aws_cfg
        .region()
        .map(ToString::to_string)
        .unwrap_or_else(|| "unknown".to_owned());
    tracing::info!(
        region,
        "AWS configuration resolved; DynamoDB and CloudWatch clients ready"
    );

    // --- Broadcast bus ---
    // The initial receiver is dropped immediately; real subscribers attach via
    // `bus.subscribe()` per SSE connection inside `events_stream`.
    let (bus, _rx) = broadcast::channel::<Event>(NOTIFICATION_CHANNEL_CAPACITY);

    let apps = AppsCtx {
        bus,
        dynamo,
        cloudwatch,
        gate,
        table,
        public_collections: Arc::new(public_collections),
    };

    let state = AppState {
        site,
        hook_token,
        apps,
        recent: Arc::new(Mutex::new(RecentIds::with_capacity(RECENT_IDS_CAPACITY))),
    };

    Router::new()
        .route("/", get(index))
        .route("/courses", get(index))
        .route("/courses/{slug}", get(course_page))
        .route("/courses/{slug}/{session}", get(session_page))
        .route("/courses/{slug}/{session}/slides", get(slides_page))
        .route("/static/{file}", get(static_file))
        .route("/favicon.ico", get(favicon))
        .route("/hooks/notifications", post(notifications_hook))
        .route("/events", post(events_hook))
        .route("/events/config", get(events_config))
        .route("/events/verify", get(events_verify))
        .route("/events/stream", get(events_stream))
        .route("/state/{collection}/{key}", get(state_read))
        .route("/health", get(health))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers — imperative shell
// ---------------------------------------------------------------------------

/// SNS delivery endpoint. Confirms subscriptions; folds notification events
/// onto the unified bus as `courses_core::Event` with kind `"notification"`.
async fn notifications_hook(
    State(state): State<AppState>,
    Query(auth): Query<HookAuth>,
    body: String,
) -> Response {
    if let Some(expected) = state.hook_token.as_deref() {
        let provided = auth.token.as_deref().unwrap_or_default();
        if !token_matches(expected, provided) {
            tracing::warn!("rejected /hooks/notifications: bad or missing token");
            return StatusCode::UNAUTHORIZED.into_response();
        }
    }
    match parse_sns_message(&body) {
        Ok(SnsMessage::Confirmation { subscribe_url }) => {
            tokio::spawn(confirm_subscription(subscribe_url));
            StatusCode::OK.into_response()
        }
        Ok(SnsMessage::Event(notification)) => {
            emit_notification(&state.apps.bus, &notification);
            StatusCode::OK.into_response()
        }
        Err(e) => {
            tracing::warn!("ignoring SNS payload: {e}");
            StatusCode::BAD_REQUEST.into_response()
        }
    }
}

/// Builds a `courses_core::Event` from a `Notification` and sends it on the bus.
fn emit_notification(bus: &broadcast::Sender<Event>, notification: &Notification) {
    // Use the notification's pod field as the id base so it is guaranteed
    // non-empty. Notifications are sent directly on the bus and bypass the
    // `RecentIds` dedup used by `POST /events`, so the id only needs to be
    // non-empty (not globally unique).
    let raw_id = format!("notif-{}", notification.pod);
    let id = match EventId::parse(raw_id) {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("could not build notification event id: {e}");
            return;
        }
    };
    match serde_json::to_string(notification) {
        Ok(payload) => {
            let event = Event {
                id,
                kind: "notification".into(),
                payload,
            };
            // A send error only means no SSE clients are connected; ignore.
            let _ = bus.send(event);
        }
        Err(e) => tracing::error!("failed to serialize notification: {e}"),
    }
}

/// Confirms an SNS subscription by GETting the one-time `SubscribeURL`.
async fn confirm_subscription(subscribe_url: String) {
    match reqwest::get(&subscribe_url).await {
        Ok(response) => tracing::info!("confirmed SNS subscription: {}", response.status()),
        Err(e) => tracing::error!("failed to confirm SNS subscription: {e}"),
    }
}

/// Generic event ingestion endpoint. Parses, deduplicates, gates, and dispatches.
async fn events_hook(
    State(state): State<AppState>,
    Query(auth): Query<AppSecret>,
    body: String,
) -> Response {
    // Parse
    let event = match parse_event(&body) {
        Ok(ev) => ev,
        Err(e) => {
            tracing::warn!("rejected /events: malformed body: {e}");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    // Dedup via Mutex<RecentIds> — recover from poison rather than panic.
    let seen = {
        let mut guard = match state.recent.lock() {
            Ok(g) => g,
            Err(poison) => {
                tracing::error!("RecentIds mutex poisoned — recovering");
                poison.into_inner()
            }
        };
        let current = std::mem::take(&mut *guard);
        let (next, seen) = current.observe(&event.id);
        *guard = next;
        seen
    };

    if seen == Seen::Duplicate {
        tracing::info!(
            event_id = %event.id.as_str(),
            event_type = %event.kind,
            "ignoring duplicate event id (idempotency); already seen in the recent window"
        );
        return StatusCode::ACCEPTED.into_response();
    }

    match courses_apps::dispatch(&state.apps, event, auth.secret.as_deref()).await {
        Outcome::Accepted => StatusCode::ACCEPTED.into_response(),
        Outcome::Unknown => StatusCode::OK.into_response(),
        Outcome::Denied => StatusCode::FORBIDDEN.into_response(),
    }
}

/// Client-facing apps config. Currently only advertises whether handlers are
/// gated, so the client knows whether to show the instructor unlock form.
async fn events_config(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "gated": state.apps.gate.requires_secret() }))
}

/// Validates an instructor secret without dispatching anything. The unlock
/// modal calls this before storing the secret, so a wrong value never unlocks
/// the UI. Returns 204 on match, 403 otherwise.
async fn events_verify(State(state): State<AppState>, Query(auth): Query<AppSecret>) -> StatusCode {
    if state.apps.gate.accepts(auth.secret.as_deref()) {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::FORBIDDEN
    }
}

/// Unified server-sent events stream carrying all `courses_core::Event` values.
async fn events_stream(
    State(bus): State<broadcast::Sender<Event>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<SseEvent, Infallible>>> {
    let stream = BroadcastStream::new(bus.subscribe()).filter_map(|message| match message {
        Ok(event) => match serde_json::to_string(&event) {
            Ok(json) => Some(Ok(SseEvent::default().data(json))),
            Err(e) => {
                tracing::error!("failed to serialize event for SSE: {e}");
                None
            }
        },
        // Intentional design: a lagged slow client sees a gap in the stream
        // rather than an error. This matches the notifications behavior — the
        // client recovers by re-rendering state on the next event it does receive.
        Err(_) => None,
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Read-only state endpoint. Gated by the public-collections whitelist.
async fn state_read(
    State(state): State<AppState>,
    Path((collection, key)): Path<(String, String)>,
) -> Response {
    match courses_apps::read_item(&state.apps, &collection, &key).await {
        Ok(Some(value)) => Json(value).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("state_read error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn health() -> &'static str {
    "ok"
}

/// Serves the favicon at the conventional root path browsers request directly.
async fn favicon() -> Response {
    bytes("image/x-icon", FAVICON_ICO)
}

async fn index(State(site): State<Arc<RenderedSite>>) -> Html<String> {
    Html(site.index_html.clone())
}

async fn course_page(State(site): State<Arc<RenderedSite>>, Path(slug): Path<String>) -> Response {
    // Pages are keyed by parsed slugs, so any invalid slug simply misses.
    lookup_page(&site, &slug)
}

async fn session_page(
    State(site): State<Arc<RenderedSite>>,
    Path((slug, session)): Path<(String, String)>,
) -> Response {
    lookup_page(&site, &format!("{slug}/{session}"))
}

async fn slides_page(
    State(site): State<Arc<RenderedSite>>,
    Path((slug, session)): Path<(String, String)>,
) -> Response {
    lookup_page(&site, &format!("{slug}/{session}/slides"))
}

fn lookup_page(site: &RenderedSite, key: &str) -> Response {
    match site.pages.get(key) {
        Some(html) => Html(html.clone()).into_response(),
        None => not_found(site),
    }
}

// The {file} param cannot contain a literal slash (the router rejects it at
// match time), so path traversal via `..` is impossible here.
async fn static_file(State(site): State<Arc<RenderedSite>>, Path(file): Path<String>) -> Response {
    match file.as_str() {
        "toggle.js" => asset("application/javascript; charset=utf-8", TOGGLE_JS),
        "cb-widgets.css" => asset("text/css; charset=utf-8", CB_WIDGETS_CSS),
        "guide.css" => asset("text/css; charset=utf-8", GUIDE_CSS),
        "reveal.min.js" => asset("application/javascript; charset=utf-8", REVEAL_JS),
        "reveal.min.css" => asset("text/css; charset=utf-8", REVEAL_CSS),
        "slides.css" => asset("text/css; charset=utf-8", SLIDES_CSS),
        "favicon.png" => bytes("image/png", FAVICON_PNG),
        "favicon.ico" => bytes("image/x-icon", FAVICON_ICO),
        "mermaid.min.js" => asset("application/javascript; charset=utf-8", MERMAID_JS),
        "mermaid-init.js" => asset("application/javascript; charset=utf-8", MERMAID_INIT_JS),
        "apps.js" => asset("application/javascript; charset=utf-8", APPS_JS),
        "montserrat.ttf" => bytes("font/ttf", MONTSERRAT_TTF),
        "cloudbridge.png" => bytes("image/png", CLOUDBRIDGE_PNG),
        "cloudbridge-white.png" => bytes("image/png", CLOUDBRIDGE_WHITE_PNG),
        _ => not_found(&site),
    }
}

fn bytes(content_type: &'static str, body: &'static [u8]) -> Response {
    ([(header::CONTENT_TYPE, content_type)], body).into_response()
}

fn asset(content_type: &'static str, body: &'static str) -> Response {
    ([(header::CONTENT_TYPE, content_type)], body).into_response()
}

fn not_found(site: &RenderedSite) -> Response {
    (StatusCode::NOT_FOUND, Html(site.not_found_html.clone())).into_response()
}

const TOGGLE_JS: &str = include_str!("../static/toggle.js");
const CB_WIDGETS_CSS: &str = include_str!("../static/cb-widgets.css");
const GUIDE_CSS: &str = include_str!("../static/guide.css");
const REVEAL_JS: &str = include_str!("../static/reveal.min.js");
const REVEAL_CSS: &str = include_str!("../static/reveal.min.css");
const SLIDES_CSS: &str = include_str!("../static/slides.css");
const MERMAID_JS: &str = include_str!("../static/mermaid.min.js");
const MERMAID_INIT_JS: &str = include_str!("../static/mermaid-init.js");
const FAVICON_ICO: &[u8] = include_bytes!("../static/favicon.ico");
const FAVICON_PNG: &[u8] = include_bytes!("../static/favicon.png");
const MONTSERRAT_TTF: &[u8] = include_bytes!("../static/montserrat.ttf");
const CLOUDBRIDGE_PNG: &[u8] = include_bytes!("../static/cloudbridge.png");
const CLOUDBRIDGE_WHITE_PNG: &[u8] = include_bytes!("../static/cloudbridge-white.png");
