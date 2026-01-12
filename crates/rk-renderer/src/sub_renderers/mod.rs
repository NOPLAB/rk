//! Built-in sub-renderers implementing the SubRenderer trait.
//!
//! These renderers provide the core visual elements for the URDF editor:
//! - Grid: Ground reference plane
//! - Mesh: 3D geometry rendering
//! - Axis: Coordinate frame indicators
//! - Marker: Joint point visualization
//! - Gizmo: Transform manipulation tool

mod grid;

pub use grid::GridSubRenderer;

// Re-export priorities for reference
pub mod priorities {
    /// Grid is rendered first (background)
    pub const GRID: i32 = 0;
    /// Meshes are the main content
    pub const MESH: i32 = 100;
    /// Axes are rendered on top of meshes
    pub const AXIS: i32 = 200;
    /// Markers are rendered on top of axes
    pub const MARKER: i32 = 300;
    /// Gizmo is always on top
    pub const GIZMO: i32 = 1000;
}
