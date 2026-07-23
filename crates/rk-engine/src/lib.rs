//! Headless CAD engine for RK.
//!
//! Owns the document (parts/assembly + CAD sketches and feature history),
//! applies serializable commands, and emits events describing what changed.
//! GUI and agent frontends are both clients of this crate.

mod command;
mod document;
mod engine;
mod error;
mod event;
mod exec;
mod history;

pub use command::{Command, ExtrudePreviewRequest, PrimitiveSpec};
pub use document::{DOCUMENT_VERSION, Document, DocumentError};
pub use engine::{Engine, InteractionId, SharedEngine};
pub use error::EngineError;
pub use event::{Event, ResetReason};
pub use history::JournalEntry;
