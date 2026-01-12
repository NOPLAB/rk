//! Core traits for the renderer system.
//!
//! This module defines the foundational traits that enable a plugin-based
//! renderer architecture.

mod render_pass;
mod sub_renderer;

pub use render_pass::*;
pub use sub_renderer::*;
