//! Pose type definition

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Pose (position and orientation)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Pose {
    pub xyz: [f32; 3],
    pub rpy: [f32; 3], // roll, pitch, yaw in radians
}

impl Pose {
    pub fn new(xyz: [f32; 3], rpy: [f32; 3]) -> Self {
        Self { xyz, rpy }
    }

    pub fn from_position(xyz: [f32; 3]) -> Self {
        Self { xyz, rpy: [0.0; 3] }
    }

    pub fn to_mat4(&self) -> Mat4 {
        let translation = Vec3::from(self.xyz);
        let rotation = Quat::from_euler(glam::EulerRot::XYZ, self.rpy[0], self.rpy[1], self.rpy[2]);
        Mat4::from_rotation_translation(rotation, translation)
    }

    /// Convert to quaternion representation
    pub fn to_quat(&self) -> Quat {
        Quat::from_euler(glam::EulerRot::XYZ, self.rpy[0], self.rpy[1], self.rpy[2])
    }

    /// Get position as Vec3
    pub fn position(&self) -> Vec3 {
        Vec3::from(self.xyz)
    }
}

impl From<&urdf_rs::Pose> for Pose {
    fn from(urdf_pose: &urdf_rs::Pose) -> Self {
        Self {
            xyz: [
                urdf_pose.xyz.0[0] as f32,
                urdf_pose.xyz.0[1] as f32,
                urdf_pose.xyz.0[2] as f32,
            ],
            rpy: [
                urdf_pose.rpy.0[0] as f32,
                urdf_pose.rpy.0[1] as f32,
                urdf_pose.rpy.0[2] as f32,
            ],
        }
    }
}
