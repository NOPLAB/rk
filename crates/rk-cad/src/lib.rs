//! CAD Kernel Abstraction and Sketch System
//!
//! This crate provides:
//! - Abstract CAD kernel traits for geometry operations
//! - 2D sketch system with entities and constraints
//! - Constraint solver using Newton-Raphson iteration
//! - Feature operations (extrude, revolve, boolean)
//! - Parametric history for design changes

pub mod feature;
pub mod history;
pub mod kernel;
pub mod sketch;

// Re-exports for convenience
pub use feature::{BooleanOp, CadBody, ExtrudeDirection, Feature, FeatureError, FeatureResult};
pub use history::{CadData, FeatureHistory, HistoryEntry};
pub use kernel::{
    Axis3D, BooleanType, CadError, CadKernel, CadResult, NullKernel, Solid, TessellatedMesh,
    Wire2D, default_kernel,
};
pub use sketch::{
    ConstraintSolver, Sketch, SketchConstraint, SketchEntity, SketchError, SketchPlane, SolveResult,
};
