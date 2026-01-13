//! World transform calculations for Assembly

use std::collections::HashMap;

use glam::{Mat4, Quat};
use uuid::Uuid;

use crate::part::JointType;

use super::Assembly;

impl Assembly {
    /// Get the world transform of a link
    pub fn get_world_transform(&self, link_id: Uuid) -> Mat4 {
        let mut transform = Mat4::IDENTITY;
        let mut current = Some(link_id);

        // Build transform chain from root to link
        let mut chain = Vec::new();
        while let Some(id) = current {
            chain.push(id);
            current = self.parent.get(&id).map(|(_, p)| *p);
        }

        // Apply transforms from root to link
        for id in chain.into_iter().rev() {
            if let Some((joint_id, _)) = self.parent.get(&id)
                && let Some(joint) = self.joints.get(joint_id)
            {
                transform *= joint.origin.to_mat4();
            }
        }

        transform
    }

    /// Update all world transforms
    pub fn update_world_transforms(&mut self) {
        let roots = self.get_root_links();
        for root_id in roots {
            self.update_transform_recursive(root_id, Mat4::IDENTITY);
        }
    }

    fn update_transform_recursive(&mut self, link_id: Uuid, parent_transform: Mat4) {
        let transform = if let Some((joint_id, _)) = self.parent.get(&link_id) {
            if let Some(joint) = self.joints.get(joint_id) {
                parent_transform * joint.origin.to_mat4()
            } else {
                parent_transform
            }
        } else {
            parent_transform
        };

        if let Some(link) = self.links.get_mut(&link_id) {
            link.world_transform = transform;
        }

        // Get children IDs first to avoid borrow issues
        let children: Vec<Uuid> = self
            .children
            .get(&link_id)
            .map(|c| c.iter().map(|(_, child_id)| *child_id).collect())
            .unwrap_or_default();

        for child_id in children {
            self.update_transform_recursive(child_id, transform);
        }
    }

    /// Update all world transforms with joint positions applied
    pub fn update_world_transforms_with_positions(&mut self, joint_positions: &HashMap<Uuid, f32>) {
        let roots = self.get_root_links();
        for root_id in roots {
            self.update_transform_recursive_with_positions(
                root_id,
                Mat4::IDENTITY,
                joint_positions,
            );
        }
    }

    fn update_transform_recursive_with_positions(
        &mut self,
        link_id: Uuid,
        parent_transform: Mat4,
        joint_positions: &HashMap<Uuid, f32>,
    ) {
        let transform = if let Some((joint_id, _)) = self.parent.get(&link_id) {
            if let Some(joint) = self.joints.get(joint_id) {
                // Get joint position (defaults to 0)
                let position = joint_positions.get(joint_id).copied().unwrap_or(0.0);
                // Compute joint transform with position
                let joint_transform =
                    Self::compute_joint_transform(&joint.joint_type, joint.axis, position);
                parent_transform * joint.origin.to_mat4() * joint_transform
            } else {
                parent_transform
            }
        } else {
            parent_transform
        };

        if let Some(link) = self.links.get_mut(&link_id) {
            link.world_transform = transform;
        }

        // Get children IDs first to avoid borrow issues
        let children: Vec<Uuid> = self
            .children
            .get(&link_id)
            .map(|c| c.iter().map(|(_, child_id)| *child_id).collect())
            .unwrap_or_default();

        for child_id in children {
            self.update_transform_recursive_with_positions(child_id, transform, joint_positions);
        }
    }

    /// Compute the transform for a joint at a given position
    pub fn compute_joint_transform(
        joint_type: &JointType,
        axis: glam::Vec3,
        position: f32,
    ) -> Mat4 {
        match joint_type {
            JointType::Revolute | JointType::Continuous => {
                // Rotation around the joint axis
                let rotation = Quat::from_axis_angle(axis, position);
                Mat4::from_quat(rotation)
            }
            JointType::Prismatic => {
                // Translation along the joint axis
                let translation = axis * position;
                Mat4::from_translation(translation)
            }
            JointType::Fixed | JointType::Floating | JointType::Planar => {
                // No transform for fixed joints, floating/planar would need more DOFs
                Mat4::IDENTITY
            }
        }
    }

    /// Update all world transforms using internal joint positions
    pub fn update_world_transforms_with_current_positions(&mut self) {
        let roots = self.get_root_links();
        for root_id in roots {
            self.update_transform_recursive_with_positions(
                root_id,
                Mat4::IDENTITY,
                &self.joint_positions.clone(),
            );
        }
    }
}
