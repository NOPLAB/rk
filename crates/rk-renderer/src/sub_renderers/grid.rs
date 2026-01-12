//! Grid sub-renderer implementing the SubRenderer trait.

use crate::constants::grid as constants;
use crate::context::RenderContext;
use crate::pipeline::PipelineConfig;
use crate::scene::Scene;
use crate::traits::SubRenderer;
use crate::vertex::PositionColorVertex;

/// Grid sub-renderer for ground reference plane.
pub struct GridSubRenderer {
    enabled: bool,
    initialized: bool,
    pipeline: Option<wgpu::RenderPipeline>,
    vertex_buffer: Option<wgpu::Buffer>,
    camera_bind_group: Option<wgpu::BindGroup>,
    vertex_count: u32,
}

impl GridSubRenderer {
    /// Creates a new grid sub-renderer.
    pub fn new() -> Self {
        Self {
            enabled: true,
            initialized: false,
            pipeline: None,
            vertex_buffer: None,
            camera_bind_group: None,
            vertex_count: 0,
        }
    }
}

impl Default for GridSubRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SubRenderer for GridSubRenderer {
    fn name(&self) -> &str {
        "grid"
    }

    fn priority(&self) -> i32 {
        super::priorities::GRID
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn on_init(&mut self, ctx: &RenderContext) {
        let pipeline = PipelineConfig::new(
            "Grid",
            include_str!("../shaders/grid.wgsl"),
            ctx.surface_format(),
            ctx.depth_format(),
            &[ctx.camera_bind_group_layout()],
        )
        .with_vertex_layouts(vec![PositionColorVertex::layout()])
        .with_topology(wgpu::PrimitiveTopology::LineList)
        .build(ctx.device());

        // Generate grid vertices
        let vertices = generate_grid_vertices(constants::DEFAULT_SIZE, constants::DEFAULT_SPACING);
        self.vertex_count = vertices.len() as u32;

        let vertex_buffer = ctx.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create bind group referencing the shared camera buffer
        let camera_bind_group = ctx.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grid Camera Bind Group"),
            layout: ctx.camera_bind_group_layout(),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ctx.camera_buffer().as_entire_binding(),
            }],
        });

        self.pipeline = Some(pipeline);
        self.vertex_buffer = Some(vertex_buffer);
        self.camera_bind_group = Some(camera_bind_group);
        self.initialized = true;
    }

    fn on_resize(&mut self, _ctx: &RenderContext, _width: u32, _height: u32) {
        // Grid doesn't need to respond to resize
    }

    fn prepare(&mut self, _ctx: &RenderContext, _scene: &Scene) {
        // Grid doesn't need per-frame preparation
    }

    fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, _scene: &Scene) {
        if !self.initialized {
            return;
        }

        let pipeline = self.pipeline.as_ref().unwrap();
        let vertex_buffer = self.vertex_buffer.as_ref().unwrap();
        let camera_bind_group = self.camera_bind_group.as_ref().unwrap();

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, camera_bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
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
