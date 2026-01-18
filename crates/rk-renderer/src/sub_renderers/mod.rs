//! Built-in sub-renderers for the URDF editor.
//!
//! This module contains all rendering components organized by functionality:
//!
//! ## New Architecture (SubRenderer trait)
//! - [`GridSubRenderer`]: Grid sub-renderer implementing the new trait
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

// Legacy implementations (to be migrated to SubRenderer trait)
pub mod axis;
pub mod collision;
pub mod gizmo;
pub mod grid_legacy;
pub mod marker;
pub mod mesh;

// Re-exports for new architecture
pub use grid::GridSubRenderer;

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
    /// Meshes are the main content
    pub const MESH: i32 = 100;
    /// Axes are rendered on top of meshes
    pub const AXIS: i32 = 200;
    /// Markers are rendered on top of axes
    pub const MARKER: i32 = 300;
    /// Gizmo is always on top
    pub const GIZMO: i32 = 1000;
}
