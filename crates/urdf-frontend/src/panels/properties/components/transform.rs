//! Transform component - position and rotation editing

use egui::Ui;
use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::panels::properties::helpers::{rotation_row, vector3_readonly_row, vector3_row};
use crate::panels::properties::{PropertyComponent, PropertyContext};

/// Transform component (position, rotation)
pub struct TransformComponent;

impl TransformComponent {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TransformComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyComponent for TransformComponent {
    fn name(&self) -> &str {
        "Transform"
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &mut PropertyContext) -> bool {
        let part = &mut ctx.part;

        // Extract position and rotation from the transform matrix (world coordinates)
        let (scale, rotation, translation) = part.origin_transform.to_scale_rotation_translation();
        let euler = rotation.to_euler(EulerRot::XYZ);

        let mut pos = [translation.x, translation.y, translation.z];
        let mut rot_deg = [
            euler.0.to_degrees(),
            euler.1.to_degrees(),
            euler.2.to_degrees(),
        ];

        // World Position (editable)
        let pos_changed = vector3_row(ui, "Position", &mut pos, 0.01);

        // Local Position (relative to parent, read-only) - only show if parent exists
        if let Some(parent_transform) = ctx.parent_world_transform {
            // Calculate local position: parent_inverse * world_position
            let parent_inverse = parent_transform.inverse();
            let local_transform = parent_inverse * part.origin_transform;
            let (_, _, local_translation) = local_transform.to_scale_rotation_translation();
            let local_pos = [
                local_translation.x,
                local_translation.y,
                local_translation.z,
            ];
            vector3_readonly_row(ui, "Local Position", &local_pos);
        }

        // Rotation
        let rot_changed = rotation_row(ui, "Rotation", &mut rot_deg, 1.0);

        // Update transform if changed
        if pos_changed || rot_changed {
            let new_rotation = Quat::from_euler(
                EulerRot::XYZ,
                rot_deg[0].to_radians(),
                rot_deg[1].to_radians(),
                rot_deg[2].to_radians(),
            );
            let new_translation = Vec3::new(pos[0], pos[1], pos[2]);
            part.origin_transform =
                Mat4::from_scale_rotation_translation(scale, new_rotation, new_translation);
            true
        } else {
            false
        }
    }
}
