//! CAD body mesh management for persistent display.

use std::collections::HashMap;

use glam::Mat4;
use uuid::Uuid;

use crate::sub_renderers::{MeshData, MeshRenderer};

use super::MeshEntry;

/// Manages CAD body meshes for persistent display.
pub struct CadBodyManager {
    /// Map of body ID to mesh entry.
    bodies: HashMap<Uuid, MeshEntry>,
}

impl Default for CadBodyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CadBodyManager {
    /// Create a new CAD body manager.
    pub fn new() -> Self {
        Self {
            bodies: HashMap::new(),
        }
    }

    /// Add a CAD body mesh for persistent display.
    ///
    /// CAD bodies are rendered like regular parts, but managed separately.
    #[allow(clippy::too_many_arguments)]
    pub fn add(
        &mut self,
        device: &wgpu::Device,
        mesh_renderer: &MeshRenderer,
        body_id: Uuid,
        vertices: &[[f32; 3]],
        normals: &[[f32; 3]],
        indices: &[u32],
        transform: Mat4,
        color: [f32; 4],
    ) {
        let data = MeshData::from_arrays(device, vertices, normals, indices, transform, color);
        let bind_group = mesh_renderer.create_instance_bind_group(device, &data);
        self.bodies.insert(body_id, MeshEntry { data, bind_group });
        tracing::info!(
            "Added CAD body: {}. Total CAD bodies: {}",
            body_id,
            self.bodies.len()
        );
    }

    /// Remove a CAD body.
    pub fn remove(&mut self, body_id: Uuid) {
        self.bodies.remove(&body_id);
    }

    /// Clear all CAD bodies.
    pub fn clear(&mut self) {
        self.bodies.clear();
    }

    /// Check if a CAD body exists.
    pub fn has(&self, body_id: Uuid) -> bool {
        self.bodies.contains_key(&body_id)
    }

    /// Get the number of CAD bodies.
    pub fn count(&self) -> usize {
        self.bodies.len()
    }

    /// Get an iterator over all CAD body entries.
    pub fn iter(&self) -> impl Iterator<Item = &MeshEntry> {
        self.bodies.values()
    }

    /// Check if there are any CAD bodies.
    pub fn is_empty(&self) -> bool {
        self.bodies.is_empty()
    }
}
