//! Coordinate axis gizmo renderer

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::util::DeviceExt;

/// Axis instance data - passed as vertex instance
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct AxisInstance {
    pub transform: [[f32; 4]; 4],
    pub scale: f32,
    pub _padding: [f32; 3],
}

impl Default for AxisInstance {
    fn default() -> Self {
        Self {
            transform: Mat4::IDENTITY.to_cols_array_2d(),
            scale: 1.0,
            _padding: [0.0; 3],
        }
    }
}

/// Axis renderer for coordinate frame visualization
pub struct AxisRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    instance_buffer: wgpu::Buffer,
    instance_count: u32,
    max_instances: u32,
    bind_group: wgpu::BindGroup,
}

impl AxisRenderer {
    const MAX_INSTANCES: u32 = 64;

    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Axis Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/axis.wgsl").into()),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Axis Camera Bind Group"),
            layout: camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Axis Pipeline Layout"),
            bind_group_layouts: &[camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Axis Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    // Vertex buffer
                    wgpu::VertexBufferLayout {
                        array_stride: 24,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: 12,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                        ],
                    },
                    // Instance buffer
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<AxisInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            // transform column 0
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            // transform column 1
                            wgpu::VertexAttribute {
                                offset: 16,
                                shader_location: 3,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            // transform column 2
                            wgpu::VertexAttribute {
                                offset: 32,
                                shader_location: 4,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            // transform column 3
                            wgpu::VertexAttribute {
                                offset: 48,
                                shader_location: 5,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            // scale + padding
                            wgpu::VertexAttribute {
                                offset: 64,
                                shader_location: 6,
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
                topology: wgpu::PrimitiveTopology::LineList,
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

        // Generate axis vertices (X=red, Y=green, Z=blue)
        let vertices = generate_axis_vertices();
        let vertex_count = vertices.len() as u32 / 6;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Axis Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Axis Instance Buffer"),
            size: (Self::MAX_INSTANCES as usize * std::mem::size_of::<AxisInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            vertex_buffer,
            vertex_count,
            instance_buffer,
            instance_count: 0,
            max_instances: Self::MAX_INSTANCES,
            bind_group,
        }
    }

    /// Update axis instances
    pub fn update_instances(&mut self, queue: &wgpu::Queue, instances: &[AxisInstance]) {
        let count = instances.len().min(self.max_instances as usize);
        self.instance_count = count as u32;
        if count > 0 {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances[..count]));
        }
    }

    /// Add a single axis at the given transform
    pub fn set_single_axis(&mut self, queue: &wgpu::Queue, transform: Mat4, scale: f32) {
        let instance = AxisInstance {
            transform: transform.to_cols_array_2d(),
            scale,
            _padding: [0.0; 3],
        };
        self.update_instances(queue, &[instance]);
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.instance_count == 0 {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw(0..self.vertex_count, 0..self.instance_count);
    }
}

fn generate_axis_vertices() -> Vec<f32> {
    let mut vertices = Vec::new();

    // X axis (red)
    vertices.extend_from_slice(&[0.0, 0.0, 0.0]);
    vertices.extend_from_slice(&[1.0, 0.0, 0.0]);
    vertices.extend_from_slice(&[1.0, 0.0, 0.0]);
    vertices.extend_from_slice(&[1.0, 0.0, 0.0]);

    // Y axis (green)
    vertices.extend_from_slice(&[0.0, 0.0, 0.0]);
    vertices.extend_from_slice(&[0.0, 1.0, 0.0]);
    vertices.extend_from_slice(&[0.0, 1.0, 0.0]);
    vertices.extend_from_slice(&[0.0, 1.0, 0.0]);

    // Z axis (blue)
    vertices.extend_from_slice(&[0.0, 0.0, 0.0]);
    vertices.extend_from_slice(&[0.0, 0.0, 1.0]);
    vertices.extend_from_slice(&[0.0, 0.0, 1.0]);
    vertices.extend_from_slice(&[0.0, 0.0, 1.0]);

    vertices
}
