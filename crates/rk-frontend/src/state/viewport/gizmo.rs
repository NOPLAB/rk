//! Gizmo interaction state

use glam::{Mat4, Quat, Vec3};
use rk_renderer::GizmoAxis;
use uuid::Uuid;

/// Gizmo transform result from drag operation
#[derive(Clone, Copy)]
pub enum GizmoTransform {
    Translation(Vec3),
    Rotation(Quat),
    Scale(Vec3),
}

/// Gizmo interaction state
#[derive(Default)]
pub struct GizmoInteraction {
    pub dragging: bool,
    pub drag_axis: GizmoAxis,
    pub drag_start_pos: Vec3,
    pub drag_start_angle: f32,
    pub part_start_transform: Mat4,
    pub part_id: Option<Uuid>,
    /// Collision being edited: (link_id, collision_index)
    pub editing_collision: Option<(Uuid, usize)>,
    /// Link world transform for collision editing
    pub link_world_transform: Mat4,
    pub gizmo_position: Vec3,
    pub gizmo_scale: f32,
}
