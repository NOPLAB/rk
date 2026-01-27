//! Built-in sub-renderers for the URDF editor.
//!
//! This module contains all rendering components organized by functionality:
//!
//! ## New Architecture (SubRenderer trait)
//! - [`GridSubRenderer`]: Grid sub-renderer implementing the new trait
//! - [`SketchRenderer`]: 2D sketch visualization on 3D planes
//! - [`PlaneSelectorRenderer`]: Reference plane selection for sketch creation
//!
//! ## Legacy Renderers (being migrated)
//! - [`grid_legacy::GridRenderer`]: Legacy grid implementation
//! - [`mesh::MeshRenderer`]: 3D geometry rendering
//! - [`axis::AxisRenderer`]: Coordinate frame indicators
//! - [`marker::MarkerRenderer`]: Joint point visualization
//! - [`gizmo::GizmoRenderer`]: Transform manipulation tool
//! - [`collision::CollisionRenderer`]: Collision shape visualization

// New trait-based implementations
mod grid;
pub mod plane_selector;
pub mod sketch;

// Legacy implementations (to be migrated to SubRenderer trait)
pub mod axis;
pub mod collision;
pub mod gizmo;
pub mod grid_legacy;
pub mod marker;
pub mod mesh;

// Re-exports for new architecture
pub use grid::GridSubRenderer;
pub use plane_selector::{PlaneSelectorRenderer, PlaneSelectorVertex, plane_ids};
pub use sketch::{ConstraintIconData, SketchRenderData, SketchRenderer, SketchVertex};

// Re-exports for legacy code
pub use axis::{AxisInstance, AxisRenderer};
pub use collision::{CollisionInstance, CollisionRenderer};
pub use gizmo::{GizmoAxis, GizmoMode, GizmoRenderer, GizmoSpace};
pub use grid_legacy::GridRenderer;
pub use marker::{MarkerInstance, MarkerRenderer};
pub use mesh::{MeshData, MeshRenderer, MeshVertex};

/// Render priorities for sub-renderers.
///
/// Lower values are rendered first (background), higher values are rendered
/// on top. Use these constants when implementing custom sub-renderers.
pub mod priorities {
    /// Grid is rendered first (background)
    pub const GRID: i32 = 0;
    /// Sketches are rendered after grid, before meshes
    pub const SKETCH: i32 = 50;
    /// Meshes are the main content
    pub const MESH: i32 = 100;
    /// Axes are rendered on top of meshes
    pub const AXIS: i32 = 200;
    /// Markers are rendered on top of axes
    pub const MARKER: i32 = 300;
    /// Gizmo is always on top
    pub const GIZMO: i32 = 1000;
}
