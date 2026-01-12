//! Joint point marker renderer

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::constants::{instances, marker as constants};
use crate::instanced::InstanceBuffer;
use crate::pipeline::{PipelineConfig, create_camera_bind_group};
use crate::vertex::PositionVertex;

/// Marker instance data - passed as vertex instance
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MarkerInstance {
    pub position: [f32; 3],
    pub radius: f32,
    pub color: [f32; 4],
}

impl MarkerInstance {
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
    pipeline: wgpu::RenderPipeline,
    /// Pipeline for selected markers (always on top, no depth test)
    selected_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    instances: InstanceBuffer<MarkerInstance>,
    /// Selected marker instances (rendered on top)
    selected_instances: InstanceBuffer<MarkerInstance>,
    bind_group: wgpu::BindGroup,
}

impl MarkerRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) -> Self {
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
        let index_count = indices.len() as u32;

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

        let instances = InstanceBuffer::new(device, "Marker", instances::MAX_MARKERS);
        let selected_instances =
            InstanceBuffer::new(device, "Selected Marker", instances::MAX_MARKERS);

        Self {
            pipeline,
            selected_pipeline,
            vertex_buffer,
            index_buffer,
            index_count,
            instances,
            selected_instances,
            bind_group,
        }
    }

    /// Update marker instances
    pub fn update_instances(&mut self, queue: &wgpu::Queue, instances: &[MarkerInstance]) {
        self.instances.update(queue, instances);
    }

    /// Update selected marker instances (rendered on top)
    pub fn update_selected_instances(&mut self, queue: &wgpu::Queue, instances: &[MarkerInstance]) {
        self.selected_instances.update(queue, instances);
    }

    /// Clear all markers
    pub fn clear(&mut self) {
        self.instances.clear();
        self.selected_instances.clear();
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        // Render normal markers first (with depth test)
        if !self.instances.is_empty() {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instances.slice());
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.index_count, 0, 0..self.instances.count());
        }

        // Render selected markers on top (no depth test)
        if !self.selected_instances.is_empty() {
            render_pass.set_pipeline(&self.selected_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.selected_instances.slice());
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.index_count, 0, 0..self.selected_instances.count());
        }
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
