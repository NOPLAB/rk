//! Built-in sub-renderers for the URDF editor.
//!
//! This module contains all rendering components organized by functionality:
//!
//! ## SubRenderer trait implementations
//! - [`GridSubRenderer`]: Ground grid renderer
//! - [`SketchRenderer`]: 2D sketch visualization on 3D planes
//! - [`PlaneSelectorRenderer`]: Reference plane selection for sketch creation
//! - [`AxisRenderer`]: Coordinate frame indicators
//! - [`MarkerRenderer`]: Joint point visualization
//! - [`CollisionRenderer`]: Collision shape visualization
//! - [`GizmoRenderer`]: Transform manipulation tool
//!
//! ## Special renderers
//! - [`MeshRenderer`]: 3D geometry rendering (supports shadow pass)

// Sub-renderer implementations
pub mod axis;
pub mod collision;
pub mod gizmo;
pub mod grid;
pub mod marker;
pub mod mesh;
pub mod plane_selector;
pub mod sketch;

// Re-exports for new architecture
pub use grid::GridSubRenderer;
pub use plane_selector::{PlaneSelectorRenderer, PlaneSelectorVertex, plane_ids};
pub use sketch::{
    ArcData, ConstraintIconData, DimensionLine, SketchRenderData, SketchRenderer, SketchVertex,
};

// Re-exports for legacy code
pub use axis::{AxisInstance, AxisRenderer};
pub use collision::{CollisionInstance, CollisionRenderer};
pub use gizmo::{GizmoAxis, GizmoMode, GizmoRenderer, GizmoSpace};
pub use marker::{MarkerInstance, MarkerRenderer};
pub use mesh::{MeshData, MeshRenderer, MeshVertex};

/// Grid renderer alias for backward compatibility (uses GridSubRenderer).
pub type GridRenderer = GridSubRenderer;

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
    /// Collision shapes are rendered semi-transparent
    pub const COLLISION: i32 = 350;
    /// Plane selector for sketch plane selection
    pub const PLANE_SELECTOR: i32 = 400;
    /// Gizmo is always on top
    pub const GIZMO: i32 = 1000;
}
