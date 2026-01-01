//! Joint point marker renderer

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

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

/// Marker renderer for joint points
pub struct MarkerRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    instance_buffer: wgpu::Buffer,
    instance_count: u32,
    max_instances: u32,
    bind_group: wgpu::BindGroup,
}

impl MarkerRenderer {
    const MAX_INSTANCES: u32 = 256;

    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Marker Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/marker.wgsl").into()),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Marker Camera Bind Group"),
            layout: camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Marker Pipeline Layout"),
            bind_group_layouts: &[camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Marker Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    // Vertex buffer
                    wgpu::VertexBufferLayout {
                        array_stride: 12, // vec3
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        }],
                    },
                    // Instance buffer
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<MarkerInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            // position + radius
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            // color
                            wgpu::VertexAttribute {
                                offset: 16,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                        ],
                    },
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Generate sphere mesh
        let (vertices, indices) = generate_sphere(16, 12);
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

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Marker Instance Buffer"),
            size: (Self::MAX_INSTANCES as usize * std::mem::size_of::<MarkerInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            index_count,
            instance_buffer,
            instance_count: 0,
            max_instances: Self::MAX_INSTANCES,
            bind_group,
        }
    }

    /// Update marker instances
    pub fn update_instances(&mut self, queue: &wgpu::Queue, instances: &[MarkerInstance]) {
        let count = instances.len().min(self.max_instances as usize);
        self.instance_count = count as u32;
        if count > 0 {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances[..count]));
        }
    }

    /// Clear all markers
    pub fn clear(&mut self) {
        self.instance_count = 0;
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.instance_count == 0 {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, 0..self.instance_count);
    }
}

/// Generate a unit sphere mesh
fn generate_sphere(segments: u32, rings: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
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

            vertices.push([x, y, z]);
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
