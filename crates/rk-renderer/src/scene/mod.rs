//! Scene management for renderable objects.
//!
//! This module separates scene data management from rendering logic,
//! enabling cleaner architecture and better extensibility.

mod bounds;
mod render_object;

pub use bounds::*;
pub use render_object::*;

use std::collections::HashMap;
use uuid::Uuid;

/// Scene containing all renderable objects.
///
/// The scene is the single source of truth for object state,
/// providing a clear separation from the rendering system.
pub struct Scene {
    objects: HashMap<Uuid, RenderObject>,
    selected: Option<Uuid>,
    dirty: bool,
}

impl Scene {
    /// Creates a new empty scene.
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            selected: None,
            dirty: false,
        }
    }

    /// Returns true if the scene has been modified since last render.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Marks the scene as clean (called after rendering).
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Marks the scene as dirty (needs re-render).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Adds an object to the scene.
    pub fn add_object(&mut self, object: RenderObject) -> Uuid {
        let id = object.id;
        self.objects.insert(id, object);
        self.dirty = true;
        id
    }

    /// Gets an object by ID.
    pub fn get_object(&self, id: Uuid) -> Option<&RenderObject> {
        self.objects.get(&id)
    }

    /// Gets a mutable reference to an object by ID.
    pub fn get_object_mut(&mut self, id: Uuid) -> Option<&mut RenderObject> {
        self.dirty = true;
        self.objects.get_mut(&id)
    }

    /// Removes an object from the scene.
    pub fn remove_object(&mut self, id: Uuid) -> Option<RenderObject> {
        if self.selected == Some(id) {
            self.selected = None;
        }
        self.dirty = true;
        self.objects.remove(&id)
    }

    /// Returns true if the scene contains an object with the given ID.
    pub fn contains(&self, id: Uuid) -> bool {
        self.objects.contains_key(&id)
    }

    /// Returns the number of objects in the scene.
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Returns true if the scene is empty.
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Clears all objects from the scene.
    pub fn clear(&mut self) {
        self.objects.clear();
        self.selected = None;
        self.dirty = true;
    }

    /// Returns an iterator over all objects.
    pub fn objects(&self) -> impl Iterator<Item = &RenderObject> {
        self.objects.values()
    }

    /// Returns a mutable iterator over all objects.
    pub fn objects_mut(&mut self) -> impl Iterator<Item = &mut RenderObject> {
        self.dirty = true;
        self.objects.values_mut()
    }

    /// Gets the currently selected object ID.
    pub fn selected(&self) -> Option<Uuid> {
        self.selected
    }

    /// Sets the selected object.
    pub fn set_selected(&mut self, id: Option<Uuid>) {
        // Clear previous selection
        if let Some(prev_id) = self.selected
            && let Some(obj) = self.objects.get_mut(&prev_id)
        {
            obj.selected = false;
        }

        // Set new selection
        self.selected = id;
        if let Some(new_id) = id
            && let Some(obj) = self.objects.get_mut(&new_id)
        {
            obj.selected = true;
        }

        self.dirty = true;
    }

    /// Gets the selected object.
    pub fn selected_object(&self) -> Option<&RenderObject> {
        self.selected.and_then(|id| self.objects.get(&id))
    }

    /// Computes the bounding box of all visible objects.
    pub fn compute_bounds(&self) -> Option<BoundingBox> {
        let mut result: Option<BoundingBox> = None;

        for obj in self.objects.values() {
            if !obj.visible {
                continue;
            }

            // Transform the object's bounds
            let transformed_bounds = obj.bounds.transform(&obj.transform);

            result = Some(match result {
                Some(current) => current.union(&transformed_bounds),
                None => transformed_bounds,
            });
        }

        result
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
