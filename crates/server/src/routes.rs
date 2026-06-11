use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use courses_core::RenderedSite;

const TOGGLE_JS: &str = include_str!("../static/toggle.js");
const GUIDE_CSS: &str = include_str!("../static/guide.css");
const REVEAL_JS: &str = include_str!("../static/reveal.min.js");
const REVEAL_CSS: &str = include_str!("../static/reveal.min.css");
const SLIDES_CSS: &str = include_str!("../static/slides.css");
const MONTSERRAT_TTF: &[u8] = include_bytes!("../static/montserrat.ttf");
const CLOUDBRIDGE_PNG: &[u8] = include_bytes!("../static/cloudbridge.png");
const CLOUDBRIDGE_WHITE_PNG: &[u8] = include_bytes!("../static/cloudbridge-white.png");

/// Builds the application router over the pre-rendered site.
pub fn router(site: Arc<RenderedSite>) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/courses", get(index))
        .route("/courses/{slug}", get(course_page))
        .route("/courses/{slug}/{session}", get(session_page))
        .route("/courses/{slug}/{session}/slides", get(slides_page))
        .route("/static/{file}", get(static_file))
        .route("/health", get(health))
        .with_state(site)
}

async fn health() -> &'static str {
    "ok"
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
