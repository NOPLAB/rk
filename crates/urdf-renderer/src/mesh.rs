//! STL mesh renderer

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::util::DeviceExt;

use urdf_core::Part;

/// Vertex for mesh rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}

/// Mesh instance transform
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MeshInstance {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub selected: u32,
    pub _padding: [u32; 3],
}

impl Default for MeshInstance {
    fn default() -> Self {
        Self {
            model: Mat4::IDENTITY.to_cols_array_2d(),
            color: [0.7, 0.7, 0.7, 1.0],
            selected: 0,
            _padding: [0; 3],
        }
    }
}

/// GPU mesh data
pub struct MeshData {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub instance: MeshInstance,
    pub instance_buffer: wgpu::Buffer,
}

impl MeshData {
    /// Create mesh data from a Part
    pub fn from_part(device: &wgpu::Device, part: &Part) -> Self {
        // Build vertices with normals
        let mut vertices = Vec::new();

        for (i, chunk) in part.indices.chunks(3).enumerate() {
            if chunk.len() != 3 {
                continue;
            }

            let normal = if i < part.normals.len() {
                part.normals[i]
            } else {
                [0.0, 0.0, 1.0]
            };

            for &idx in chunk {
                let pos = part.vertices[idx as usize];
                vertices.push(MeshVertex {
                    position: pos,
                    normal,
                    color: part.color,
                });
            }
        }

        let indices: Vec<u32> = (0..vertices.len() as u32).collect();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance = MeshInstance {
            model: part.origin_transform.to_cols_array_2d(),
            color: part.color,
            selected: 0,
            _padding: [0; 3],
        };

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Instance Buffer"),
            contents: bytemuck::cast_slice(&[instance]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            instance,
            instance_buffer,
        }
    }

    /// Update instance transform
    pub fn update_transform(&mut self, queue: &wgpu::Queue, transform: Mat4) {
        self.instance.model = transform.to_cols_array_2d();
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&[self.instance]));
    }

    /// Update instance color
    pub fn update_color(&mut self, queue: &wgpu::Queue, color: [f32; 4]) {
        self.instance.color = color;
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&[self.instance]));
    }

    /// Set selected state
    pub fn set_selected(&mut self, queue: &wgpu::Queue, selected: bool) {
        self.instance.selected = if selected { 1 } else { 0 };
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&[self.instance]));
    }
}

/// Mesh renderer
pub struct MeshRenderer {
    pipeline: wgpu::RenderPipeline,
    camera_bind_group: wgpu::BindGroup,
    instance_bind_group_layout: wgpu::BindGroupLayout,
}

impl MeshRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Mesh Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/mesh.wgsl").into()),
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mesh Camera Bind Group"),
            layout: camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let instance_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Mesh Instance Bind Group Layout"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh Pipeline Layout"),
            bind_group_layouts: &[camera_bind_group_layout, &instance_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Mesh Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex>() as u64,
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
                        wgpu::VertexAttribute {
                            offset: 24,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
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

        Self {
            pipeline,
            camera_bind_group,
            instance_bind_group_layout,
        }
    }

    /// Create bind group for a mesh instance
    pub fn create_instance_bind_group(&self, device: &wgpu::Device, mesh: &MeshData) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mesh Instance Bind Group"),
            layout: &self.instance_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: mesh.instance_buffer.as_entire_binding(),
            }],
        })
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        mesh: &'a MeshData,
        instance_bind_group: &'a wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, instance_bind_group, &[]);
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}
