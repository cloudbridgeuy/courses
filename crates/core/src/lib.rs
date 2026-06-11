#![deny(clippy::unwrap_used, clippy::expect_used)]

//! Pure domain for the courses platform: course model, and HTML rendering.
//!
//! This crate is a functional core: no I/O, no async, no environment access.

mod course;
mod error;
mod render;

pub use course::{Course, CourseSlug, GuideSection};
pub use error::{Error, Result};
pub use render::{escape_html, render_guide_page};
