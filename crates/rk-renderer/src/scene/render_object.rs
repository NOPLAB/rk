//! Render object definition.

use glam::Mat4;
use uuid::Uuid;

use super::BoundingBox;
use crate::resources::MeshHandle;

/// A renderable object in the scene.
///
/// RenderObject contains all the data needed to render an object,
/// separated from the GPU resources which are managed by resource managers.
#[derive(Debug, Clone)]
pub struct RenderObject {
    /// Unique identifier for this object.
    pub id: Uuid,

    /// Handle to the mesh data stored in MeshManager.
    pub mesh: MeshHandle,

    /// World transform matrix.
    pub transform: Mat4,

    /// Base color (RGBA).
    pub color: [f32; 4],

    /// Whether this object is visible.
    pub visible: bool,

    /// Whether this object is selected.
    pub selected: bool,

    /// Local bounding box (before transform).
    pub bounds: BoundingBox,

    /// Render layer for sorting and filtering.
    pub layer: RenderLayer,
}

impl RenderObject {
    /// Creates a new render object with default settings.
    pub fn new(id: Uuid, mesh: MeshHandle, bounds: BoundingBox) -> Self {
        Self {
            id,
            mesh,
            transform: Mat4::IDENTITY,
            color: [0.8, 0.8, 0.8, 1.0],
            visible: true,
            selected: false,
            bounds,
            layer: RenderLayer::Default,
        }
    }

    /// Sets the transform matrix.
    pub fn with_transform(mut self, transform: Mat4) -> Self {
        self.transform = transform;
        self
    }

    /// Sets the color.
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// Sets the visibility.
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Sets the render layer.
    pub fn with_layer(mut self, layer: RenderLayer) -> Self {
        self.layer = layer;
        self
    }

    /// Returns the world-space bounding box.
    pub fn world_bounds(&self) -> BoundingBox {
        self.bounds.transform(&self.transform)
    }
}

/// Render layer for sorting and filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RenderLayer {
    /// Default opaque geometry layer.
    #[default]
    Default,
    /// Transparent geometry (rendered back-to-front).
    Transparent,
    /// Overlay elements (rendered on top).
    Overlay,
    /// Custom layer with user-defined ID.
    Custom(u32),
}

impl RenderLayer {
    /// Returns the sort order for this layer (lower = rendered first).
    pub fn sort_order(&self) -> i32 {
        match self {
            RenderLayer::Default => 0,
            RenderLayer::Transparent => 100,
            RenderLayer::Overlay => 200,
            RenderLayer::Custom(id) => 1000 + (*id as i32),
        }
    }

    /// Returns true if this layer uses alpha blending.
    pub fn uses_blending(&self) -> bool {
        matches!(self, RenderLayer::Transparent | RenderLayer::Overlay)
    }
}
