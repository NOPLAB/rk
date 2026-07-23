//! Headless CAD engine for RK.
//!
//! Owns the document (parts/assembly + CAD sketches and feature history),
//! applies serializable commands, and emits events describing what changed.
//! GUI and agent frontends are both clients of this crate.

mod document;

pub use document::{DOCUMENT_VERSION, Document, DocumentError};
