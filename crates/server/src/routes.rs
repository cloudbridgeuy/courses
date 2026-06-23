use std::convert::Infallible;
use std::sync::Arc;

use axum::Router;
use axum::extract::{FromRef, Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use courses_core::{RenderedSite, SnsMessage, parse_sns_message, token_matches};
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

const NOTIFICATIONS_JS: &str = include_str!("../static/notifications.js");

/// Buffer of in-flight toast events per subscriber before a slow client lags.
const NOTIFICATION_CHANNEL_CAPACITY: usize = 256;

/// Environment variable holding the shared webhook secret. When set, the SNS
/// ingestion endpoint requires a matching `?token=` (the unguessable-URL pattern
/// chat webhooks use). When unset, the endpoint is open — dev convenience only.
const HOOK_TOKEN_ENV: &str = "CB_HOOK_TOKEN";

/// Shared handler state: the pre-rendered site, the toast broadcast channel, and
/// the optional webhook secret.
///
/// `FromRef` lets page handlers keep extracting `State<Arc<RenderedSite>>` while
/// hook handlers extract `State<broadcast::Sender<String>>`.
#[derive(Clone)]
pub struct AppState {
    site: Arc<RenderedSite>,
    notifier: broadcast::Sender<String>,
    hook_token: Option<Arc<String>>,
}

/// The `?token=` query on the ingestion endpoint.
#[derive(Deserialize)]
struct HookAuth {
    token: Option<String>,
}

impl FromRef<AppState> for Arc<RenderedSite> {
    fn from_ref(state: &AppState) -> Self {
        Arc::clone(&state.site)
    }
}

impl FromRef<AppState> for broadcast::Sender<String> {
    fn from_ref(state: &AppState) -> Self {
        state.notifier.clone()
    }
}

const TOGGLE_JS: &str = include_str!("../static/toggle.js");
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

/// Builds the application router over the pre-rendered site.
pub fn router(site: Arc<RenderedSite>) -> Router {
    let (notifier, _rx) = broadcast::channel::<String>(NOTIFICATION_CHANNEL_CAPACITY);
    let hook_token = std::env::var(HOOK_TOKEN_ENV).ok().map(Arc::new);
    if hook_token.is_none() {
        tracing::warn!(
            "{HOOK_TOKEN_ENV} is unset: /hooks/notifications is open (no token required)"
        );
    }
    let state = AppState {
        site,
        notifier,
        hook_token,
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
        .route("/hooks/stream", get(notifications_stream))
        .route("/health", get(health))
        .with_state(state)
}

/// SNS delivery endpoint. Requires a matching `?token=` when a secret is
/// configured. On a subscription handshake it confirms by fetching the
/// `SubscribeURL`; on an event it broadcasts the JSON to every SSE client.
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
    let notifier = &state.notifier;
    match parse_sns_message(&body) {
        Ok(SnsMessage::Confirmation { subscribe_url }) => {
            tokio::spawn(confirm_subscription(subscribe_url));
            StatusCode::OK.into_response()
        }
        Ok(SnsMessage::Event(notification)) => {
            match serde_json::to_string(&notification) {
                // A send error only means no SSE clients are connected; ignore.
                Ok(json) => {
                    let _ = notifier.send(json);
                }
                Err(e) => tracing::error!("failed to serialize notification: {e}"),
            }
            StatusCode::OK.into_response()
        }
        Err(e) => {
            tracing::warn!("ignoring SNS payload: {e}");
            StatusCode::BAD_REQUEST.into_response()
        }
    }
}

/// Confirms an SNS subscription by GETting the one-time `SubscribeURL`.
async fn confirm_subscription(subscribe_url: String) {
    match reqwest::get(&subscribe_url).await {
        Ok(response) => tracing::info!("confirmed SNS subscription: {}", response.status()),
        Err(e) => tracing::error!("failed to confirm SNS subscription: {e}"),
    }
}

/// Server-sent events stream of toast notifications for the guide UI.
async fn notifications_stream(
    State(notifier): State<broadcast::Sender<String>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(notifier.subscribe()).filter_map(|message| match message {
        Ok(data) => Some(Ok(Event::default().data(data))),
        // A lagged receiver dropped messages; skip the gap rather than error out.
        Err(_) => None,
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
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
        "guide.css" => asset("text/css; charset=utf-8", GUIDE_CSS),
        "reveal.min.js" => asset("application/javascript; charset=utf-8", REVEAL_JS),
        "reveal.min.css" => asset("text/css; charset=utf-8", REVEAL_CSS),
        "slides.css" => asset("text/css; charset=utf-8", SLIDES_CSS),
        "favicon.png" => bytes("image/png", FAVICON_PNG),
        "favicon.ico" => bytes("image/x-icon", FAVICON_ICO),
        "mermaid.min.js" => asset("application/javascript; charset=utf-8", MERMAID_JS),
        "mermaid-init.js" => asset("application/javascript; charset=utf-8", MERMAID_INIT_JS),
        "notifications.js" => asset("application/javascript; charset=utf-8", NOTIFICATIONS_JS),
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
