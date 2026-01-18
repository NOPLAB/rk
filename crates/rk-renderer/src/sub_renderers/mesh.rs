//! STL mesh renderer with shadow mapping support

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::util::DeviceExt;

use rk_core::Part;

use crate::constants::viewport::SAMPLE_COUNT;
use crate::pipeline::create_camera_bind_group;

/// Vertex for mesh rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MeshVertex {
    /// Vertex position in local space.
    pub position: [f32; 3],
    /// Vertex normal vector.
    pub normal: [f32; 3],
    /// Vertex color (RGBA).
    pub color: [f32; 4],
}

impl MeshVertex {
    /// Vertex attribute descriptors for the shader.
    pub const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &[
        wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x3,
        },
        wgpu::VertexAttribute {
            offset: std::mem::size_of::<[f32; 3]>() as u64,
            shader_location: 1,
            format: wgpu::VertexFormat::Float32x3,
        },
        wgpu::VertexAttribute {
            offset: (std::mem::size_of::<[f32; 3]>() * 2) as u64,
            shader_location: 2,
            format: wgpu::VertexFormat::Float32x4,
        },
    ];

    /// Returns the vertex buffer layout for this vertex type.
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

/// Mesh instance transform
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MeshInstance {
    /// Model transformation matrix.
    pub model: [[f32; 4]; 4],
    /// Instance color (RGBA).
    pub color: [f32; 4],
    /// Selection state (0 = unselected, 1 = selected).
    pub selected: u32,
    /// Padding for alignment.
    pub _pad: [u32; 3],
}

impl Default for MeshInstance {
    fn default() -> Self {
        Self {
            model: Mat4::IDENTITY.to_cols_array_2d(),
            color: [0.7, 0.7, 0.7, 1.0],
            selected: 0,
            _pad: [0; 3],
        }
    }
}

/// GPU mesh data
pub struct MeshData {
    /// Vertex buffer containing mesh geometry.
    pub vertex_buffer: wgpu::Buffer,
    /// Index buffer for indexed drawing.
    pub index_buffer: wgpu::Buffer,
    /// Number of indices.
    pub index_count: u32,
    /// Instance data (transform, color, selection).
    pub instance: MeshInstance,
    /// GPU buffer for instance data.
    pub instance_buffer: wgpu::Buffer,
}

impl MeshData {
    /// Create mesh data from a Part
    pub fn from_part(device: &wgpu::Device, part: &Part) -> Self {
        tracing::info!(
            "Creating MeshData: {} vertices, {} normals, {} indices, bbox_min={:?}, bbox_max={:?}",
            part.vertices.len(),
            part.normals.len(),
            part.indices.len(),
            part.bbox_min,
            part.bbox_max
        );

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

        tracing::info!(
            "MeshData created: {} GPU vertices, {} indices",
            vertices.len(),
            indices.len()
        );

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
            _pad: [0; 3],
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

    /// Create mesh data from vertex/normal/index arrays (for preview meshes)
    pub fn from_arrays(
        device: &wgpu::Device,
        mesh_vertices: &[[f32; 3]],
        mesh_normals: &[[f32; 3]],
        mesh_indices: &[u32],
        transform: Mat4,
        color: [f32; 4],
    ) -> Self {
        // Build vertices with normals
        let mut vertices = Vec::new();

        for (i, chunk) in mesh_indices.chunks(3).enumerate() {
            if chunk.len() != 3 {
                continue;
            }

            // Get face normal - try per-vertex normals first, then compute from vertices
            let normal = if i * 3 < mesh_normals.len() {
                mesh_normals[i * 3]
            } else if !mesh_normals.is_empty() && (chunk[0] as usize) < mesh_normals.len() {
                mesh_normals[chunk[0] as usize]
            } else {
                // Compute face normal from vertex positions
                let p0 = mesh_vertices
                    .get(chunk[0] as usize)
                    .copied()
                    .unwrap_or([0.0, 0.0, 0.0]);
                let p1 = mesh_vertices
                    .get(chunk[1] as usize)
                    .copied()
                    .unwrap_or([0.0, 0.0, 0.0]);
                let p2 = mesh_vertices
                    .get(chunk[2] as usize)
                    .copied()
                    .unwrap_or([0.0, 0.0, 0.0]);
                let v0 = glam::Vec3::from(p0);
                let v1 = glam::Vec3::from(p1);
                let v2 = glam::Vec3::from(p2);
                let normal = (v1 - v0).cross(v2 - v0).normalize_or_zero();
                normal.to_array()
            };

            for &idx in chunk {
                let pos = mesh_vertices
                    .get(idx as usize)
                    .copied()
                    .unwrap_or([0.0, 0.0, 0.0]);
                vertices.push(MeshVertex {
                    position: pos,
                    normal,
                    color,
                });
            }
        }

        let indices: Vec<u32> = (0..vertices.len() as u32).collect();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Preview Mesh Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Preview Mesh Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance = MeshInstance {
            model: transform.to_cols_array_2d(),
            color,
            selected: 0,
            _pad: [0; 3],
        };

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Preview Mesh Instance Buffer"),
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
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&[self.instance]),
        );
    }

    /// Update instance color
    pub fn update_color(&mut self, queue: &wgpu::Queue, color: [f32; 4]) {
        self.instance.color = color;
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&[self.instance]),
        );
    }

    /// Set selected state
    pub fn set_selected(&mut self, queue: &wgpu::Queue, selected: bool) {
        self.instance.selected = if selected { 1 } else { 0 };
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&[self.instance]),
        );
    }
}

/// Mesh renderer with shadow mapping support
pub struct MeshRenderer {
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    camera_bind_group: wgpu::BindGroup,
    instance_bind_group_layout: wgpu::BindGroupLayout,
    light_bind_group_layout: wgpu::BindGroupLayout,
}

impl MeshRenderer {
    /// Creates a new mesh renderer with shadow mapping support.
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Mesh Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/mesh.wgsl").into()),
        });

        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shadow Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/shadow.wgsl").into()),
        });

        let camera_bind_group =
            create_camera_bind_group(device, camera_bind_group_layout, camera_buffer, "Mesh");

        // Per-mesh instance bind group layout (for transform/color/selection)
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

        // Light + shadow bind group layout (group 2)
        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Light Bind Group Layout"),
                entries: &[
                    // Light uniform buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Shadow map texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Shadow sampler (comparison)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
            });

        // Main pipeline layout with 3 bind groups: camera, instance, light+shadow
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh Pipeline Layout"),
            bind_group_layouts: &[
                camera_bind_group_layout,
                &instance_bind_group_layout,
                &light_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Mesh Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[MeshVertex::layout()],
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
                cull_mode: None, // Disable culling to show both sides
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Shadow pipeline - uses light uniform at group 0, instance at group 1
        // (different from main pipeline which has camera at group 0)
        let shadow_light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shadow Light Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Shadow Pipeline Layout"),
                bind_group_layouts: &[&shadow_light_bind_group_layout, &instance_bind_group_layout],
                push_constant_ranges: &[],
            });

        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shadow Pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
                entry_point: Some("vs_main"),
                buffers: &[MeshVertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shadow_shader,
                entry_point: Some("fs_main"),
                targets: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back), // Cull back faces for shadow pass
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(), // No MSAA for shadow map
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            shadow_pipeline,
            camera_bind_group,
            instance_bind_group_layout,
            light_bind_group_layout,
        }
    }

    /// Get the light bind group layout
    pub fn light_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.light_bind_group_layout
    }

    /// Get the instance bind group layout
    pub fn instance_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.instance_bind_group_layout
    }

    /// Create bind group for a mesh instance
    pub fn create_instance_bind_group(
        &self,
        device: &wgpu::Device,
        mesh: &MeshData,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mesh Instance Bind Group"),
            layout: &self.instance_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: mesh.instance_buffer.as_entire_binding(),
            }],
        })
    }

    /// Render mesh to shadow map (depth-only pass)
    pub fn render_shadow<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        mesh: &'a MeshData,
        instance_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.shadow_pipeline);
        render_pass.set_bind_group(0, light_bind_group, &[]);
        render_pass.set_bind_group(1, instance_bind_group, &[]);
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }

    /// Render mesh with lighting and shadows
    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        mesh: &'a MeshData,
        instance_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, instance_bind_group, &[]);
        render_pass.set_bind_group(2, light_bind_group, &[]);
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}
