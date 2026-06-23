#![deny(clippy::unwrap_used, clippy::expect_used)]

//! Pure domain for the courses platform: course model, content pipeline, and
//! HTML rendering.
//!
//! This crate is a functional core: no I/O, no async, no environment access.

mod assets;
mod catalog;
mod course;
mod error;
mod events;
mod manifest;
mod markdown;
mod notifications;
mod render;
mod section;
mod slides;
mod solutions;

pub use assets::{
    APPS_JS_PATH, CB_WIDGETS_CSS_PATH, GUIDE_CSS_PATH, PageAssets, REVEAL_CSS_PATH, REVEAL_JS_PATH,
    SLIDES_CSS_PATH, TOGGLE_JS_PATH,
};
pub use catalog::{CourseInput, LoadedCourse, parse_course};
pub use course::{Course, CourseSlug, GuideSection, Session, SessionSlug};
pub use error::{Error, Result};
pub use events::{
    CpuBurstConfig, Decision, Event, EventId, Gate, GateConfig, HandlerKind, Intensity, RecentIds,
    Seen, gate, is_public_collection, parse_event, parse_gate, select,
};
pub use manifest::{Manifest, parse_manifest};
pub use markdown::render_markdown;
pub use notifications::{Notification, SnsMessage, parse_sns_message, token_matches};
pub use render::{
    RenderedSite, SessionPage, escape_html, render_index_page, render_landing_page,
    render_not_found_page, render_session_page, render_site, render_slideshow_page, render_tree,
};
pub use section::{Frontmatter, parse_frontmatter, split_frontmatter};
pub use solutions::{RenderedBody, Segment, render_section_body, split_solutions};
