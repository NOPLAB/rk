//! Part mesh management.

use std::collections::HashMap;

use glam::Mat4;
use uuid::Uuid;

use rk_core::Part;

use crate::sub_renderers::{MeshData, MeshRenderer};

use super::MeshEntry;

/// Manages part meshes with UUID-keyed storage for O(1) lookup and removal.
pub struct PartManager {
    /// Map of part ID to mesh entry.
    meshes: HashMap<Uuid, MeshEntry>,
    /// Currently selected part ID.
    selected_part: Option<Uuid>,
}

impl Default for PartManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PartManager {
    /// Create a new part manager.
    pub fn new() -> Self {
        Self {
            meshes: HashMap::new(),
            selected_part: None,
        }
    }

    /// Add a part to the manager.
    ///
    /// Returns the part's UUID for reference.
    pub fn add(&mut self, device: &wgpu::Device, mesh_renderer: &MeshRenderer, part: &Part) -> Uuid {
        tracing::info!("PartManager::add called for '{}'", part.name);
        let data = MeshData::from_part(device, part);
        let bind_group = mesh_renderer.create_instance_bind_group(device, &data);

        self.meshes.insert(part.id, MeshEntry { data, bind_group });
        tracing::info!("PartManager now has {} meshes", self.meshes.len());
        part.id
    }

    /// Update a part's transform.
    pub fn update_transform(&mut self, queue: &wgpu::Queue, part_id: Uuid, transform: Mat4) {
        if let Some(entry) = self.meshes.get_mut(&part_id) {
            entry.data.update_transform(queue, transform);
        }
    }

    /// Update a part's color.
    pub fn update_color(&mut self, queue: &wgpu::Queue, part_id: Uuid, color: [f32; 4]) {
        if let Some(entry) = self.meshes.get_mut(&part_id) {
            entry.data.update_color(queue, color);
        }
    }

    /// Set selected part.
    pub fn set_selected(&mut self, queue: &wgpu::Queue, part_id: Option<Uuid>) {
        // Deselect previous
        if let Some(prev_id) = self.selected_part
            && let Some(entry) = self.meshes.get_mut(&prev_id)
        {
            entry.data.set_selected(queue, false);
        }

        // Select new
        self.selected_part = part_id;
        if let Some(id) = part_id
            && let Some(entry) = self.meshes.get_mut(&id)
        {
            entry.data.set_selected(queue, true);
        }
    }

    /// Get the currently selected part ID.
    pub fn selected(&self) -> Option<Uuid> {
        self.selected_part
    }

    /// Remove a part - O(1) operation with UUID-based storage.
    pub fn remove(&mut self, part_id: Uuid) {
        self.meshes.remove(&part_id);
        if self.selected_part == Some(part_id) {
            self.selected_part = None;
        }
    }

    /// Clear all parts.
    pub fn clear(&mut self) {
        self.meshes.clear();
        self.selected_part = None;
    }

    /// Check if a part exists.
    pub fn has(&self, part_id: Uuid) -> bool {
        self.meshes.contains_key(&part_id)
    }

    /// Get the number of parts.
    pub fn count(&self) -> usize {
        self.meshes.len()
    }

    /// Get an iterator over all mesh entries.
    pub fn iter(&self) -> impl Iterator<Item = &MeshEntry> {
        self.meshes.values()
    }

    /// Check if there are any parts.
    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty()
    }
}
