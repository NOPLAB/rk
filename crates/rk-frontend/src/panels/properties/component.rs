//! PropertyComponent trait definition for Unity-style Inspector components

use egui::Ui;
use glam::Mat4;
use rk_core::{CollisionElement, Part};
use uuid::Uuid;

use crate::state::AppAction;

/// Context passed to property components for rendering
pub struct PropertyContext<'a> {
    /// The part being edited
    pub part: &'a mut Part,
    /// Parent link's world transform (if this part has a parent in assembly)
    pub parent_world_transform: Option<Mat4>,
    /// The link's UUID (if any)
    pub link_id: Option<Uuid>,
    /// Copy of the link's collisions for display (modifications via actions)
    pub collisions: Vec<CollisionElement>,
    /// Currently selected collision index (if any)
    pub selected_collision_index: Option<usize>,
    /// Queue for actions to be processed
    pub pending_actions: &'a mut Vec<AppAction>,
}

/// Trait for property panel components (Unity-style Inspector sections)
pub trait PropertyComponent {
    /// Component display name shown in the header
    fn name(&self) -> &str;

    /// Render the component UI
    /// Returns true if any value was changed
    fn ui(&mut self, ui: &mut Ui, ctx: &mut PropertyContext) -> bool;

    /// Whether this component is collapsible (default: true)
    fn is_collapsible(&self) -> bool {
        true
    }

    /// Whether the component is open by default (default: true)
    fn default_open(&self) -> bool {
        true
    }
}
