//! Preview mesh management for extrusion previews.

use glam::Mat4;

use crate::sub_renderers::{MeshData, MeshRenderer};

use super::MeshEntry;

/// Manages preview meshes for extrusion and other previews.
pub struct PreviewManager {
    /// The current preview mesh, if any.
    preview_mesh: Option<MeshEntry>,
}

impl Default for PreviewManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewManager {
    /// Create a new preview manager.
    pub fn new() -> Self {
        Self { preview_mesh: None }
    }

    /// Set a preview mesh for extrusion preview.
    ///
    /// The mesh will be rendered with a semi-transparent appearance.
    pub fn set_preview_mesh(
        &mut self,
        device: &wgpu::Device,
        mesh_renderer: &MeshRenderer,
        vertices: &[[f32; 3]],
        normals: &[[f32; 3]],
        indices: &[u32],
        transform: Mat4,
    ) {
        // Semi-transparent blue color for preview
        let preview_color = [0.4, 0.6, 0.9, 0.5];
        let data =
            MeshData::from_arrays(device, vertices, normals, indices, transform, preview_color);
        let bind_group = mesh_renderer.create_instance_bind_group(device, &data);
        self.preview_mesh = Some(MeshEntry { data, bind_group });
    }

    /// Clear the preview mesh.
    pub fn clear(&mut self) {
        self.preview_mesh = None;
    }

    /// Check if there's a preview mesh.
    pub fn has_preview(&self) -> bool {
        self.preview_mesh.is_some()
    }

    /// Get the preview mesh entry, if any.
    pub fn preview_mesh(&self) -> Option<&MeshEntry> {
        self.preview_mesh.as_ref()
    }
}
