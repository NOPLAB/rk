//! Joint point marker renderer

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::constants::{instances, marker as constants};
use crate::context::RenderContext;
use crate::instanced::InstanceBuffer;
use crate::pipeline::{PipelineConfig, create_camera_bind_group};
use crate::scene::Scene;
use crate::traits::SubRenderer;
use crate::vertex::PositionVertex;

/// Marker instance data - passed as vertex instance
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MarkerInstance {
    /// Marker center position in world space.
    pub position: [f32; 3],
    /// Marker sphere radius.
    pub radius: f32,
    /// Marker color (RGBA).
    pub color: [f32; 4],
}

impl MarkerInstance {
    /// Creates a new marker instance.
    pub fn new(position: Vec3, radius: f32, color: [f32; 4]) -> Self {
        Self {
            position: position.to_array(),
            radius,
            color,
        }
    }
}

impl Default for MarkerInstance {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            radius: 0.02,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// Marker renderer for joint points
pub struct MarkerRenderer {
    enabled: bool,
    initialized: bool,
    pipeline: Option<wgpu::RenderPipeline>,
    /// Pipeline for selected markers (always on top, no depth test)
    selected_pipeline: Option<wgpu::RenderPipeline>,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    index_count: u32,
    instances: Option<InstanceBuffer<MarkerInstance>>,
    /// Selected marker instances (rendered on top)
    selected_instances: Option<InstanceBuffer<MarkerInstance>>,
    bind_group: Option<wgpu::BindGroup>,
}

impl MarkerRenderer {
    /// Creates a new marker renderer (uninitialized).
    pub fn new() -> Self {
        Self {
            enabled: true,
            initialized: false,
            pipeline: None,
            selected_pipeline: None,
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
            instances: None,
            selected_instances: None,
            bind_group: None,
        }
    }

    /// Initialize the marker renderer with GPU resources.
    pub fn init(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) {
        let bind_group =
            create_camera_bind_group(device, camera_bind_group_layout, camera_buffer, "Marker");

        // Instance buffer layout: position+radius (Float32x4) + color (Float32x4)
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MarkerInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };

        let instance_layout_clone = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MarkerInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };

        let pipeline = PipelineConfig::new(
            "Marker",
            include_str!("../shaders/marker.wgsl"),
            format,
            depth_format,
            &[camera_bind_group_layout],
        )
        .with_vertex_layouts(vec![PositionVertex::layout(), instance_layout])
        .with_cull_mode(Some(wgpu::Face::Back))
        .build(device);

        // Pipeline for selected markers - always on top (no depth test)
        let selected_pipeline = PipelineConfig::new(
            "Selected Marker",
            include_str!("../shaders/marker.wgsl"),
            format,
            depth_format,
            &[camera_bind_group_layout],
        )
        .with_vertex_layouts(vec![PositionVertex::layout(), instance_layout_clone])
        .with_cull_mode(Some(wgpu::Face::Back))
        .without_depth_test()
        .build(device);

        // Generate sphere mesh
        let (vertices, indices) = generate_sphere(constants::SEGMENTS, constants::RINGS);
        self.index_count = indices.len() as u32;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Marker Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Marker Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let inst = InstanceBuffer::new(device, "Marker", instances::MAX_MARKERS);
        let selected_inst = InstanceBuffer::new(device, "Selected Marker", instances::MAX_MARKERS);

        self.pipeline = Some(pipeline);
        self.selected_pipeline = Some(selected_pipeline);
        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
        self.instances = Some(inst);
        self.selected_instances = Some(selected_inst);
        self.bind_group = Some(bind_group);
        self.initialized = true;
    }

    /// Update marker instances
    pub fn update_instances(&mut self, queue: &wgpu::Queue, instances: &[MarkerInstance]) {
        if let Some(ref mut inst) = self.instances {
            inst.update(queue, instances);
        }
    }

    /// Update selected marker instances (rendered on top)
    pub fn update_selected_instances(&mut self, queue: &wgpu::Queue, instances: &[MarkerInstance]) {
        if let Some(ref mut inst) = self.selected_instances {
            inst.update(queue, instances);
        }
    }

    /// Clear all markers
    pub fn clear(&mut self) {
        if let Some(ref mut inst) = self.instances {
            inst.clear();
        }
        if let Some(ref mut inst) = self.selected_instances {
            inst.clear();
        }
    }

    /// Renders all marker instances.
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if !self.initialized {
            return;
        }

        let pipeline = self.pipeline.as_ref().unwrap();
        let selected_pipeline = self.selected_pipeline.as_ref().unwrap();
        let vertex_buffer = self.vertex_buffer.as_ref().unwrap();
        let index_buffer = self.index_buffer.as_ref().unwrap();
        let bind_group = self.bind_group.as_ref().unwrap();
        let instances = self.instances.as_ref().unwrap();
        let selected_instances = self.selected_instances.as_ref().unwrap();

        // Render normal markers first (with depth test)
        if !instances.is_empty() {
            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, instances.slice());
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.index_count, 0, 0..instances.count());
        }

        // Render selected markers on top (no depth test)
        if !selected_instances.is_empty() {
            render_pass.set_pipeline(selected_pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, selected_instances.slice());
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.index_count, 0, 0..selected_instances.count());
        }
    }
}

impl Default for MarkerRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SubRenderer for MarkerRenderer {
    fn name(&self) -> &str {
        "marker"
    }

    fn priority(&self) -> i32 {
        super::priorities::MARKER
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn on_init(&mut self, ctx: &RenderContext) {
        self.init(
            ctx.device(),
            ctx.surface_format(),
            ctx.depth_format(),
            ctx.camera_bind_group_layout(),
            ctx.camera_buffer(),
        );
    }

    fn on_resize(&mut self, _ctx: &RenderContext, _width: u32, _height: u32) {
        // Marker renderer doesn't need to respond to resize
    }

    fn prepare(&mut self, _ctx: &RenderContext, _scene: &Scene) {
        // Marker data is updated externally via update_instances
    }

    fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, _scene: &Scene) {
        MarkerRenderer::render(self, pass);
    }
}

/// Generate a unit sphere mesh
fn generate_sphere(segments: u32, rings: u32) -> (Vec<PositionVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for ring in 0..=rings {
        let phi = std::f32::consts::PI * ring as f32 / rings as f32;
        let y = phi.cos();
        let ring_radius = phi.sin();

        for seg in 0..=segments {
            let theta = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
            let x = ring_radius * theta.cos();
            let z = ring_radius * theta.sin();

            vertices.push(PositionVertex {
                position: [x, y, z],
            });
        }
    }

    // Generate indices
    for ring in 0..rings {
        for seg in 0..segments {
            let current = ring * (segments + 1) + seg;
            let next = current + segments + 1;

            indices.push(current);
            indices.push(next);
            indices.push(current + 1);

            indices.push(current + 1);
            indices.push(next);
            indices.push(next + 1);
        }
    }

    (vertices, indices)
}
