//! Transform component - position, rotation, and scale editing

use egui::Ui;
use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::panels::properties::helpers::{rotation_row, vector3_row};
use crate::panels::properties::{PropertyComponent, PropertyContext};

/// Transform component (position, rotation, scale)
pub struct TransformComponent {
    /// Whether to show local coordinates instead of world coordinates
    show_local: bool,
}

impl TransformComponent {
    pub fn new() -> Self {
        Self { show_local: false }
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
        let parent_transform = ctx.parent_world_transform;

        // Extract position, rotation, and scale from the transform matrix (world coordinates)
        let (scale, rotation, translation) = part.origin_transform.to_scale_rotation_translation();
        let euler = rotation.to_euler(EulerRot::XYZ);

        let mut pos = [translation.x, translation.y, translation.z];
        let mut rot_deg = [
            euler.0.to_degrees(),
            euler.1.to_degrees(),
            euler.2.to_degrees(),
        ];
        let mut scl = [scale.x, scale.y, scale.z];

        // Local transform variables
        let mut local_pos = [0.0f32; 3];
        let mut local_rot_deg = [0.0f32; 3];
        let mut local_scl = [1.0f32; 3];

        // Calculate local transform if parent exists
        if let Some(parent) = parent_transform {
            let parent_inverse = parent.inverse();
            let local = parent_inverse * part.origin_transform;
            let (local_scale, local_rotation, local_translation) =
                local.to_scale_rotation_translation();
            let local_euler = local_rotation.to_euler(EulerRot::XYZ);

            local_pos = [
                local_translation.x,
                local_translation.y,
                local_translation.z,
            ];
            local_rot_deg = [
                local_euler.0.to_degrees(),
                local_euler.1.to_degrees(),
                local_euler.2.to_degrees(),
            ];
            local_scl = [local_scale.x, local_scale.y, local_scale.z];
        }

        // Show toggle switch only if parent exists
        if parent_transform.is_some() {
            ui.checkbox(&mut self.show_local, "Local");
            ui.add_space(4.0);
        }

        let (pos_changed, rot_changed, scale_changed);
        let (local_pos_changed, local_rot_changed, local_scale_changed);

        if self.show_local && parent_transform.is_some() {
            // Show local coordinates
            local_pos_changed = vector3_row(ui, "Position", &mut local_pos, 0.01);
            local_rot_changed = rotation_row(ui, "Rotation", &mut local_rot_deg, 1.0);
            local_scale_changed = vector3_row(ui, "Scale", &mut local_scl, 0.01);
            pos_changed = false;
            rot_changed = false;
            scale_changed = false;
        } else {
            // Show world coordinates
            pos_changed = vector3_row(ui, "Position", &mut pos, 0.01);
            rot_changed = rotation_row(ui, "Rotation", &mut rot_deg, 1.0);
            scale_changed = vector3_row(ui, "Scale", &mut scl, 0.01);
            local_pos_changed = false;
            local_rot_changed = false;
            local_scale_changed = false;
        }

        // Update transform if changed
        if local_pos_changed || local_rot_changed || local_scale_changed {
            // Local transform was edited - compute new world transform
            let parent = parent_transform.unwrap();
            let new_local_rotation = Quat::from_euler(
                EulerRot::XYZ,
                local_rot_deg[0].to_radians(),
                local_rot_deg[1].to_radians(),
                local_rot_deg[2].to_radians(),
            );
            let new_local_translation = Vec3::new(local_pos[0], local_pos[1], local_pos[2]);
            let new_local_scale = Vec3::new(local_scl[0], local_scl[1], local_scl[2]);
            let new_local_transform = Mat4::from_scale_rotation_translation(
                new_local_scale,
                new_local_rotation,
                new_local_translation,
            );
            // world = parent * local
            part.origin_transform = parent * new_local_transform;
            true
        } else if pos_changed || rot_changed || scale_changed {
            // World transform was edited
            let new_rotation = Quat::from_euler(
                EulerRot::XYZ,
                rot_deg[0].to_radians(),
                rot_deg[1].to_radians(),
                rot_deg[2].to_radians(),
            );
            let new_translation = Vec3::new(pos[0], pos[1], pos[2]);
            let new_scale = Vec3::new(scl[0], scl[1], scl[2]);
            part.origin_transform =
                Mat4::from_scale_rotation_translation(new_scale, new_rotation, new_translation);
            true
        } else {
            false
        }
    }
}
