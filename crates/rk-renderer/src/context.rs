//! Render context that encapsulates GPU resources.
//!
//! The RenderContext hides wgpu implementation details from consumers,
//! providing a clean interface for GPU operations.

use std::sync::Arc;

use wgpu::util::DeviceExt;

use crate::camera::CameraUniform;
use crate::constants::viewport::SAMPLE_COUNT;

/// Render context containing GPU resources and state.
///
/// This struct encapsulates all wgpu resources needed for rendering,
/// hiding the implementation details from consumers.
pub struct RenderContext {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    surface_format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
    sample_count: u32,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

impl RenderContext {
    /// Creates a new render context.
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let depth_format = wgpu::TextureFormat::Depth32Float;

        // Create camera bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Create camera buffer with default uniform
        let camera_uniform = CameraUniform::default();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create camera bind group
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        Self {
            device,
            queue,
            surface_format,
            depth_format,
            sample_count: SAMPLE_COUNT,
            camera_bind_group_layout,
            camera_buffer,
            camera_bind_group,
            width,
            height,
        }
    }

    /// Returns the wgpu device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Returns the wgpu queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Returns the surface texture format.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_format
    }

    /// Returns the depth texture format.
    pub fn depth_format(&self) -> wgpu::TextureFormat {
        self.depth_format
    }

    /// Returns the MSAA sample count.
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// Returns the camera bind group layout.
    pub fn camera_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.camera_bind_group_layout
    }

    /// Returns the camera buffer.
    pub fn camera_buffer(&self) -> &wgpu::Buffer {
        &self.camera_buffer
    }

    /// Returns the camera bind group.
    pub fn camera_bind_group(&self) -> &wgpu::BindGroup {
        &self.camera_bind_group
    }

    /// Returns the current viewport width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the current viewport height.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Updates the camera uniform buffer.
    pub fn update_camera(&self, uniform: &CameraUniform) {
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[*uniform]));
    }

    /// Updates the viewport dimensions.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    /// Creates a buffer with the given descriptor.
    pub fn create_buffer(&self, desc: &wgpu::BufferDescriptor) -> wgpu::Buffer {
        self.device.create_buffer(desc)
    }

    /// Creates a buffer initialized with data.
    pub fn create_buffer_init(&self, desc: &wgpu::util::BufferInitDescriptor) -> wgpu::Buffer {
        self.device.create_buffer_init(desc)
    }

    /// Writes data to a buffer.
    pub fn write_buffer(&self, buffer: &wgpu::Buffer, offset: u64, data: &[u8]) {
        self.queue.write_buffer(buffer, offset, data);
    }

    /// Creates a texture with the given descriptor.
    pub fn create_texture(&self, desc: &wgpu::TextureDescriptor) -> wgpu::Texture {
        self.device.create_texture(desc)
    }

    /// Creates a shader module from WGSL source.
    pub fn create_shader(&self, source: &str, label: &str) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
    }

    /// Creates a render pipeline.
    pub fn create_render_pipeline(
        &self,
        desc: &wgpu::RenderPipelineDescriptor,
    ) -> wgpu::RenderPipeline {
        self.device.create_render_pipeline(desc)
    }

    /// Creates a bind group layout.
    pub fn create_bind_group_layout(
        &self,
        desc: &wgpu::BindGroupLayoutDescriptor,
    ) -> wgpu::BindGroupLayout {
        self.device.create_bind_group_layout(desc)
    }

    /// Creates a bind group.
    pub fn create_bind_group(&self, desc: &wgpu::BindGroupDescriptor) -> wgpu::BindGroup {
        self.device.create_bind_group(desc)
    }

    /// Creates a pipeline layout.
    pub fn create_pipeline_layout(
        &self,
        desc: &wgpu::PipelineLayoutDescriptor,
    ) -> wgpu::PipelineLayout {
        self.device.create_pipeline_layout(desc)
    }
}
