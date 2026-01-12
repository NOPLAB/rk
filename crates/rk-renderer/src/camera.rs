//! Orbit camera for 3D viewport

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

/// Camera uniform buffer data
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub eye: [f32; 4],
}

impl Default for CameraUniform {
    fn default() -> Self {
        let identity = glam::Mat4::IDENTITY.to_cols_array_2d();
        Self {
            view_proj: identity,
            view: identity,
            proj: identity,
            eye: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// Orbit camera
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    // Orbit state
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Camera {
    /// Create a new camera with default parameters
    pub fn new(aspect: f32) -> Self {
        let yaw = 45.0_f32.to_radians();
        let pitch = 30.0_f32.to_radians();
        let distance = 5.0;
        let target = Vec3::ZERO;

        // Calculate initial position from orbit parameters
        let x = distance * pitch.cos() * yaw.cos();
        let y = distance * pitch.cos() * yaw.sin();
        let z = distance * pitch.sin();
        let position = target + Vec3::new(x, y, z);

        Self {
            position,
            target,
            up: Vec3::Z,
            fov: 40.0_f32.to_radians(),
            aspect,
            near: 0.1,
            far: 100000.0,
            yaw,
            pitch,
            distance,
        }
    }

    /// Update aspect ratio
    pub fn update_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    /// Orbit the camera around the target
    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch =
            (self.pitch + delta_pitch).clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
        self.update_position_from_orbit();
    }

    /// Pan the camera (move target)
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let forward = (self.target - self.position).normalize();
        let right = forward.cross(self.up).normalize();
        let up = right.cross(forward).normalize();

        let scale = self.distance * 0.002;
        self.target += right * (-delta_x * scale) + up * (delta_y * scale);
        self.update_position_from_orbit();
    }

    /// Zoom the camera
    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance * (1.0 - delta * 0.1)).clamp(0.1, 10000.0);
        self.update_position_from_orbit();
    }

    /// Set field of view in degrees
    pub fn set_fov_degrees(&mut self, fov_degrees: f32) {
        self.fov = fov_degrees.clamp(10.0, 120.0).to_radians();
    }

    /// Get field of view in degrees
    pub fn fov_degrees(&self) -> f32 {
        self.fov.to_degrees()
    }

    /// Set near clipping plane
    pub fn set_near(&mut self, near: f32) {
        self.near = near.max(0.001);
    }

    /// Set far clipping plane
    pub fn set_far(&mut self, far: f32) {
        self.far = far.max(self.near + 1.0);
    }

    fn update_position_from_orbit(&mut self) {
        let x = self.distance * self.pitch.cos() * self.yaw.cos();
        let y = self.distance * self.pitch.cos() * self.yaw.sin();
        let z = self.distance * self.pitch.sin();
        self.position = self.target + Vec3::new(x, y, z);
    }

    /// Fit camera to show the given bounding sphere
    pub fn fit_all(&mut self, center: Vec3, radius: f32) {
        self.target = center;
        self.distance = (radius * 2.5).max(1.0);
        self.update_position_from_orbit();
    }

    /// Set to top view
    pub fn set_top_view(&mut self) {
        self.yaw = 0.0;
        self.pitch = 89.0_f32.to_radians();
        self.update_position_from_orbit();
    }

    /// Set to front view
    pub fn set_front_view(&mut self) {
        self.yaw = 0.0;
        self.pitch = 0.0;
        self.update_position_from_orbit();
    }

    /// Set to side view
    pub fn set_side_view(&mut self) {
        self.yaw = 90.0_f32.to_radians();
        self.pitch = 0.0;
        self.update_position_from_orbit();
    }

    /// Get view matrix
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Get projection matrix
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far)
    }

    /// Get camera uniform data
    pub fn uniform(&self) -> CameraUniform {
        let view = self.view_matrix();
        let proj = self.projection_matrix();
        let view_proj = proj * view;

        CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            proj: proj.to_cols_array_2d(),
            eye: [self.position.x, self.position.y, self.position.z, 1.0],
        }
    }

    /// Convert screen coordinates to world ray
    pub fn screen_to_ray(
        &self,
        screen_x: f32,
        screen_y: f32,
        screen_width: f32,
        screen_height: f32,
    ) -> (Vec3, Vec3) {
        // Convert to normalized device coordinates
        let ndc_x = (2.0 * screen_x / screen_width) - 1.0;
        let ndc_y = 1.0 - (2.0 * screen_y / screen_height);

        let inv_proj = self.projection_matrix().inverse();
        let inv_view = self.view_matrix().inverse();

        // Near and far points in NDC
        let near_ndc = glam::Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
        let far_ndc = glam::Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

        // Transform to view space
        let near_view = inv_proj * near_ndc;
        let far_view = inv_proj * far_ndc;
        let near_view = near_view.truncate() / near_view.w;
        let far_view = far_view.truncate() / far_view.w;

        // Transform to world space
        let near_world = (inv_view * near_view.extend(1.0)).truncate();
        let far_world = (inv_view * far_view.extend(1.0)).truncate();

        let ray_origin = near_world;
        let ray_direction = (far_world - near_world).normalize();

        (ray_origin, ray_direction)
    }
}
