//! Section-body directive parsing and rendering: `::: solucion`, `:::slide`,
//! `:::inline-slide`, `:::title-slide`, `::: warning`, `::: info`,
//! `::: extra`, `:::app`, `:::skip`, and `:::add` fences.
//!
//! [`directives`] tokenizes a body into [`Segment`]s; [`render`] turns those
//! segments into guide HTML and slide fragments.

mod directives;
mod render;

pub use directives::{Segment, SlideFragment, Visibility, split_solutions};
pub use render::{RenderedBody, render_section_body};
pub(crate) use render::{render_app, render_extra, render_info, render_solution, render_warning};
