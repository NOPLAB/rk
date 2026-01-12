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
//! # Example
//!
//! ```ignore
//! use rk_renderer::{Renderer, RendererConfig};
//!
//! let config = RendererConfig::default();
//! let renderer = Renderer::new(&render_state, config);
//!
//! // Add objects to the scene
//! let id = renderer.add_object(mesh_data, transform);
//!
//! // Render
//! renderer.render(&output_view);
//! ```

// Core abstractions
pub mod context;
pub mod plugin;
pub mod resources;
pub mod scene;
pub mod traits;

// Existing modules (to be migrated to sub_renderers)
pub mod axis;
pub mod camera;
pub mod constants;
pub mod gizmo;
pub mod grid;
pub mod instanced;
pub mod marker;
pub mod mesh;
pub mod pipeline;
pub mod renderer;
pub mod sub_renderers;
pub mod vertex;

// Re-exports for convenience
pub use camera::*;
pub use context::RenderContext;
pub use gizmo::*;
pub use plugin::{RendererPlugin, RendererRegistry};
pub use renderer::*;
pub use resources::{GpuMesh, MeshData, MeshHandle, MeshManager};
pub use scene::{BoundingBox, Frustum, RenderLayer, RenderObject, Scene};
pub use traits::{PassType, SubRenderer};
pub use vertex::MeshVertex;
