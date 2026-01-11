//! PropertyComponent trait definition for Unity-style Inspector components

use egui::Ui;
use glam::Mat4;
use urdf_core::{JointPoint, Part};
use uuid::Uuid;

/// Context passed to property components for rendering
pub struct PropertyContext<'a> {
    /// The part being edited
    pub part: &'a mut Part,
    /// Part ID (for state updates)
    pub part_id: Uuid,
    /// Selected joint point index (if any)
    pub selected_joint_point: Option<usize>,
    /// Parent link's world transform (if this part has a parent in assembly)
    pub parent_world_transform: Option<Mat4>,
    /// Joint points for this part (from Assembly)
    pub joint_points: Vec<JointPoint>,
    /// Actions to apply to joint points after rendering
    pub joint_point_actions: Vec<JointPointAction>,
}

/// Actions for joint point management
#[derive(Debug)]
pub enum JointPointAction {
    Add(JointPoint),
    Remove(Uuid),
    Update(JointPoint),
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
