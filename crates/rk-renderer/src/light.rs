//! Directional light for 3D rendering with shadow mapping support

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

use crate::constants::shadow;

/// Light uniform buffer data sent to GPU (128 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LightUniform {
    /// Light view-projection matrix for shadow mapping
    pub light_view_proj: [[f32; 4]; 4],
    /// Light direction (normalized, world space) - xyz = direction, w = unused
    pub direction: [f32; 4],
    /// Light color (RGB) and intensity (A)
    pub color_intensity: [f32; 4],
    /// Ambient color (RGB) and strength (A)
    pub ambient: [f32; 4],
    /// Shadow parameters: x = bias, y = normal_bias, z = softness, w = enabled (1.0 or 0.0)
    pub shadow_params: [f32; 4],
}

impl Default for LightUniform {
    fn default() -> Self {
        Self {
            light_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            direction: [0.5, 0.5, 1.0, 0.0],
            color_intensity: [1.0, 1.0, 1.0, 1.0],
            ambient: [1.0, 1.0, 1.0, 0.3],
            shadow_params: [shadow::DEFAULT_BIAS, shadow::DEFAULT_NORMAL_BIAS, 1.0, 1.0],
        }
    }
}

/// Directional light configuration
///
/// A directional light simulates a distant light source like the sun,
/// where all rays are parallel. This is the most common light type
/// for outdoor scenes and CAD visualization.
pub struct DirectionalLight {
    /// Light direction (normalized, pointing toward light source)
    pub direction: Vec3,
    /// Light color (RGB, 0.0-1.0)
    pub color: Vec3,
    /// Light intensity multiplier (typically 0.0-2.0)
    pub intensity: f32,
    /// Ambient light color
    pub ambient_color: Vec3,
    /// Ambient light strength (0.0-1.0)
    pub ambient_strength: f32,
    /// Shadow depth bias to prevent shadow acne
    pub shadow_bias: f32,
    /// Normal-based shadow bias for surfaces at grazing angles
    pub shadow_normal_bias: f32,
    /// PCF softness (affects shadow edge smoothness)
    pub shadow_softness: f32,
    /// Enable/disable shadow rendering
    pub shadows_enabled: bool,
    /// Orthographic projection half-size for shadow map (world units)
    pub ortho_size: f32,
    /// Near plane for shadow projection
    pub ortho_near: f32,
    /// Far plane for shadow projection
    pub ortho_far: f32,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectionalLight {
    /// Create a new directional light with default parameters
    pub fn new() -> Self {
        Self {
            direction: Vec3::new(0.5, 0.5, 1.0).normalize(),
            color: Vec3::ONE,
            intensity: 1.0,
            ambient_color: Vec3::ONE,
            ambient_strength: 0.3,
            shadow_bias: shadow::DEFAULT_BIAS,
            shadow_normal_bias: shadow::DEFAULT_NORMAL_BIAS,
            shadow_softness: 1.0,
            shadows_enabled: true,
            ortho_size: 20.0,
            ortho_near: 0.1,
            ortho_far: 100.0,
        }
    }

    /// Set light direction (will be normalized)
    pub fn set_direction(&mut self, dir: Vec3) {
        self.direction = dir.normalize();
    }

    /// Set light direction from yaw and pitch angles (in radians)
    pub fn set_direction_from_angles(&mut self, yaw: f32, pitch: f32) {
        let x = pitch.cos() * yaw.cos();
        let y = pitch.cos() * yaw.sin();
        let z = pitch.sin();
        self.direction = Vec3::new(x, y, z).normalize();
    }

    /// Compute the light's view matrix (looking from light toward scene center)
    pub fn view_matrix(&self, scene_center: Vec3) -> Mat4 {
        // Position light far away in the light direction
        let light_pos = scene_center + self.direction * self.ortho_far * 0.5;
        Mat4::look_at_rh(light_pos, scene_center, Vec3::Z)
    }

    /// Compute orthographic projection matrix for shadow map
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::orthographic_rh(
            -self.ortho_size,
            self.ortho_size,
            -self.ortho_size,
            self.ortho_size,
            self.ortho_near,
            self.ortho_far,
        )
    }

    /// Get the uniform data for GPU
    pub fn uniform(&self, scene_center: Vec3) -> LightUniform {
        let view = self.view_matrix(scene_center);
        let proj = self.projection_matrix();
        let light_view_proj = proj * view;

        LightUniform {
            light_view_proj: light_view_proj.to_cols_array_2d(),
            direction: [self.direction.x, self.direction.y, self.direction.z, 0.0],
            color_intensity: [self.color.x, self.color.y, self.color.z, self.intensity],
            ambient: [
                self.ambient_color.x,
                self.ambient_color.y,
                self.ambient_color.z,
                self.ambient_strength,
            ],
            shadow_params: [
                self.shadow_bias,
                self.shadow_normal_bias,
                self.shadow_softness,
                if self.shadows_enabled { 1.0 } else { 0.0 },
            ],
        }
    }

    /// Fit the shadow projection to encompass the given bounding sphere
    pub fn fit_to_scene(&mut self, center: Vec3, radius: f32) {
        self.ortho_size = radius * 1.5;
        self.ortho_far = radius * 4.0;
        let _ = center; // Center is used in view_matrix, not stored
    }
}
