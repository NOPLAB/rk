//! Ground grid renderer

use wgpu::util::DeviceExt;

/// Grid renderer
pub struct GridRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    bind_group: wgpu::BindGroup,
}

impl GridRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Grid Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/grid.wgsl").into()),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grid Bind Group"),
            layout: camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Grid Pipeline Layout"),
            bind_group_layouts: &[camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Grid Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
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

        // Generate grid vertices
        let vertices = generate_grid_vertices(10.0, 1.0);
        let vertex_count = vertices.len() as u32 / 6; // 6 floats per vertex

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            pipeline,
            vertex_buffer,
            vertex_count,
            bind_group,
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.vertex_count, 0..1);
    }
}

/// Generate grid line vertices
fn generate_grid_vertices(size: f32, spacing: f32) -> Vec<f32> {
    let mut vertices = Vec::new();
    let half_size = size;
    let num_lines = (size / spacing) as i32;

    // Grid color (gray)
    let grid_color = [0.3, 0.3, 0.3];
    // X axis color (red)
    let x_color = [0.8, 0.2, 0.2];
    // Y axis color (green)
    let y_color = [0.2, 0.8, 0.2];

    // Lines parallel to X axis
    for i in -num_lines..=num_lines {
        let y = i as f32 * spacing;
        let color = if i == 0 { x_color } else { grid_color };

        // Start point
        vertices.extend_from_slice(&[-half_size, y, 0.0]);
        vertices.extend_from_slice(&color);
        // End point
        vertices.extend_from_slice(&[half_size, y, 0.0]);
        vertices.extend_from_slice(&color);
    }

    // Lines parallel to Y axis
    for i in -num_lines..=num_lines {
        let x = i as f32 * spacing;
        let color = if i == 0 { y_color } else { grid_color };

        // Start point
        vertices.extend_from_slice(&[x, -half_size, 0.0]);
        vertices.extend_from_slice(&color);
        // End point
        vertices.extend_from_slice(&[x, half_size, 0.0]);
        vertices.extend_from_slice(&color);
    }

    vertices
}
