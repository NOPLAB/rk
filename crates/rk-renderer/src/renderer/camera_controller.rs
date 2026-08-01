//! Camera management for the renderer.

use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::config::CameraConfig;

use super::gpu_resources;

/// Manages the camera and its GPU resources.
pub struct CameraController {
    /// The camera.
    camera: Camera,
    /// GPU buffer for camera uniforms.
    buffer: wgpu::Buffer,
    /// Bind group layout for camera.
    bind_group_layout: wgpu::BindGroupLayout,
}

impl CameraController {
    /// Create a new camera controller.
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let camera = Camera::new(width as f32 / height as f32);
        let camera_uniform = camera.uniform();

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = gpu_resources::create_camera_bind_group_layout(device);

        Self {
            camera,
            buffer,
            bind_group_layout,
        }
    }

    /// Get a reference to the camera.
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Get a mutable reference to the camera.
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    /// Get the camera buffer.
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Get the camera bind group layout.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Update the camera aspect ratio.
    pub fn update_aspect(&mut self, width: u32, height: u32) {
        self.camera.update_aspect(width as f32 / height as f32);
    }

    /// Update the camera buffer on the GPU.
    pub fn update(&self, queue: &wgpu::Queue) {
        let camera_uniform = self.camera.uniform();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
    }

    /// Apply camera configuration.
    pub fn apply_config(&mut self, config: &CameraConfig) {
        self.camera.set_fov_degrees(config.fov_degrees);
        self.camera.set_near(config.near_plane);
        self.camera.set_far(config.far_plane);
    }
}
