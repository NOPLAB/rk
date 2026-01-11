//! Ground grid renderer

use wgpu::util::DeviceExt;

use crate::constants::grid as constants;
use crate::pipeline::{PipelineConfig, create_camera_bind_group};
use crate::vertex::PositionColorVertex;

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
        let bind_group =
            create_camera_bind_group(device, camera_bind_group_layout, camera_buffer, "Grid");

        let pipeline = PipelineConfig::new(
            "Grid",
            include_str!("shaders/grid.wgsl"),
            format,
            depth_format,
            &[camera_bind_group_layout],
        )
        .with_vertex_layouts(vec![PositionColorVertex::layout()])
        .with_topology(wgpu::PrimitiveTopology::LineList)
        .build(device);

        // Generate grid vertices
        let vertices = generate_grid_vertices(constants::DEFAULT_SIZE, constants::DEFAULT_SPACING);
        let vertex_count = vertices.len() as u32;

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
fn generate_grid_vertices(size: f32, spacing: f32) -> Vec<PositionColorVertex> {
    let mut vertices = Vec::new();
    let half_size = size;
    let num_lines = (size / spacing) as i32;

    // Lines parallel to X axis
    for i in -num_lines..=num_lines {
        let y = i as f32 * spacing;
        let color = if i == 0 {
            constants::X_AXIS_COLOR
        } else {
            constants::LINE_COLOR
        };

        // Start point
        vertices.push(PositionColorVertex {
            position: [-half_size, y, 0.0],
            color,
        });
        // End point
        vertices.push(PositionColorVertex {
            position: [half_size, y, 0.0],
            color,
        });
    }

    // Lines parallel to Y axis
    for i in -num_lines..=num_lines {
        let x = i as f32 * spacing;
        let color = if i == 0 {
            constants::Y_AXIS_COLOR
        } else {
            constants::LINE_COLOR
        };

        // Start point
        vertices.push(PositionColorVertex {
            position: [x, -half_size, 0.0],
            color,
        });
        // End point
        vertices.push(PositionColorVertex {
            position: [x, half_size, 0.0],
            color,
        });
    }

    vertices
}
