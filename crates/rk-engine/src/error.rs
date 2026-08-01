//! Engine error type

use uuid::Uuid;

use crate::document::DocumentError;

/// Errors returned by [`crate::Engine::apply`] and queries
#[derive(Debug, Clone, thiserror::Error)]
pub enum EngineError {
    #[error("{kind} not found: {id}")]
    NotFound { kind: &'static str, id: Uuid },
    #[error("invalid command: {0}")]
    InvalidCommand(String),
    #[error("feature error: {0}")]
    Feature(String),
    #[error("kernel error: {0}")]
    Cad(String),
    #[error("sketch error: {0}")]
    Sketch(String),
    #[error("document error: {0}")]
    Document(#[from] DocumentError),
    #[error("IO error: {0}")]
    Io(String),
}

impl From<rk_cad::FeatureError> for EngineError {
    fn from(e: rk_cad::FeatureError) -> Self {
        Self::Feature(e.to_string())
    }
}

impl From<rk_cad::CadError> for EngineError {
    fn from(e: rk_cad::CadError) -> Self {
        Self::Cad(e.to_string())
    }
}

impl From<rk_cad::SketchError> for EngineError {
    fn from(e: rk_cad::SketchError) -> Self {
        Self::Sketch(e.to_string())
    }
}
