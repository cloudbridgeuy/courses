use axum::Router;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use courses_core::{Course, CourseSlug, GuideSection, render_guide_page};

/// Builds the application router.
pub fn router() -> Router {
    Router::new()
        .route("/", get(guide))
        .route("/health", get(health))
}

async fn health() -> &'static str {
    "ok"
}

async fn guide() -> Response {
    match sample_course() {
        Ok(course) => Html(render_guide_page(&course)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Placeholder course until the content pipeline lands.
fn sample_course() -> courses_core::Result<Course> {
    Ok(Course {
        slug: CourseSlug::parse("aws-devops")?,
        title: "Taller AWS DevOps".to_owned(),
        sections: vec![GuideSection {
            title: "Bienvenida".to_owned(),
            body_html: "<p>La plataforma del taller está en construcción. \
                        Esta página la sirve <code>courses_server</code>, renderizada \
                        por <code>courses_core</code>.</p>"
                .to_owned(),
        }],
    })
}
