//! URDF Editor Renderer
//!
//! WGPU-based 3D rendering for URDF editor.
//!
//! # Architecture
//!
//! The renderer is built on a plugin-based architecture:
//!
//! - [`traits::SubRenderer`] - Trait for implementing custom renderers
//! - [`plugin::RendererRegistry`] - Registry for managing sub-renderers
//! - [`context::RenderContext`] - GPU context abstraction
//! - [`scene::Scene`] - Scene management for renderable objects
//! - [`resources::MeshManager`] - GPU mesh resource management
//!
//! # Module Structure
//!
//! ```text
//! rk-renderer/
//! ├── traits/          # Core abstractions (SubRenderer, PassType)
//! ├── context.rs       # GPU context abstraction
//! ├── scene/           # Scene management (Scene, RenderObject, BoundingBox)
//! ├── resources/       # Resource management (MeshManager)
//! ├── plugin.rs        # Plugin system (RendererRegistry)
//! ├── sub_renderers/   # Built-in renderers (Grid, Mesh, Axis, Marker, Gizmo)
//! ├── camera.rs        # Camera system
//! ├── pipeline.rs      # Pipeline utilities
//! └── renderer.rs      # Main Renderer
//! ```

// Core abstractions
pub mod context;
pub mod plugin;
pub mod resources;
pub mod scene;
pub mod traits;

// Rendering infrastructure
pub mod camera;
pub mod constants;
pub mod instanced;
pub mod pipeline;
pub mod renderer;
pub mod sub_renderers;
pub mod vertex;

// Re-export sub-renderers for backward compatibility
pub mod axis {
    //! Axis renderer (re-exported from sub_renderers)
    pub use crate::sub_renderers::axis::*;
}
pub mod gizmo {
    //! Gizmo renderer (re-exported from sub_renderers)
    pub use crate::sub_renderers::gizmo::*;
}
pub mod grid {
    //! Grid renderer (re-exported from sub_renderers)
    pub use crate::sub_renderers::grid_legacy::*;
}
pub mod marker {
    //! Marker renderer (re-exported from sub_renderers)
    pub use crate::sub_renderers::marker::*;
}
pub mod mesh {
    //! Mesh renderer (re-exported from sub_renderers)
    pub use crate::sub_renderers::mesh::*;
}

// Re-exports for convenience
pub use camera::*;
pub use context::RenderContext;
pub use plugin::{RendererPlugin, RendererRegistry};
pub use renderer::*;
pub use resources::MeshData as ResourceMeshData;
pub use resources::{GpuMesh, MeshHandle, MeshManager};
pub use scene::{BoundingBox, Frustum, RenderLayer, RenderObject, Scene};
pub use sub_renderers::{
    AxisInstance, AxisRenderer, GizmoAxis, GizmoMode, GizmoRenderer, GridRenderer, GridSubRenderer,
    MarkerInstance, MarkerRenderer, MeshRenderer,
};
pub use traits::{PassType, SubRenderer};
pub use vertex::MeshVertex;
