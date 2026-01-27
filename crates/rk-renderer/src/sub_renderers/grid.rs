//! Grid sub-renderer implementing the SubRenderer trait.

use wgpu::util::DeviceExt;

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
    // Configuration cache for rebuild
    size: f32,
    spacing: f32,
    line_color: [f32; 3],
    x_axis_color: [f32; 3],
    y_axis_color: [f32; 3],
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
            size: constants::DEFAULT_SIZE,
            spacing: constants::DEFAULT_SPACING,
            line_color: constants::LINE_COLOR,
            x_axis_color: constants::X_AXIS_COLOR,
            y_axis_color: constants::Y_AXIS_COLOR,
        }
    }

    /// Initialize the grid renderer with GPU resources.
    ///
    /// This allows direct initialization without using SubRenderer trait.
    pub fn init(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) {
        use crate::pipeline::PipelineConfig;

        let pipeline = PipelineConfig::new(
            "Grid",
            include_str!("../shaders/grid.wgsl"),
            format,
            depth_format,
            &[camera_bind_group_layout],
        )
        .with_vertex_layouts(vec![PositionColorVertex::layout()])
        .with_topology(wgpu::PrimitiveTopology::LineList)
        .build(device);

        // Generate grid vertices
        let vertices = generate_grid_vertices(self.size, self.spacing);
        self.vertex_count = vertices.len() as u32;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create bind group referencing the shared camera buffer
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grid Camera Bind Group"),
            layout: camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        self.pipeline = Some(pipeline);
        self.vertex_buffer = Some(vertex_buffer);
        self.camera_bind_group = Some(camera_bind_group);
        self.initialized = true;
    }

    /// Rebuild grid with new parameters.
    ///
    /// This method allows changing the grid configuration after initialization.
    pub fn rebuild(
        &mut self,
        device: &wgpu::Device,
        size: f32,
        spacing: f32,
        line_color: [f32; 3],
        x_axis_color: [f32; 3],
        y_axis_color: [f32; 3],
    ) {
        self.size = size;
        self.spacing = spacing;
        self.line_color = line_color;
        self.x_axis_color = x_axis_color;
        self.y_axis_color = y_axis_color;

        let vertices = generate_grid_vertices_with_colors(
            size,
            spacing,
            line_color,
            x_axis_color,
            y_axis_color,
        );
        self.vertex_count = vertices.len() as u32;

        self.vertex_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Grid Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );
    }

    /// Renders the grid (standalone method for legacy API).
    pub fn render_legacy<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if !self.initialized {
            return;
        }

        let pipeline = self.pipeline.as_ref().unwrap();
        let vertex_buffer = self.vertex_buffer.as_ref().unwrap();
        let camera_bind_group = self.camera_bind_group.as_ref().unwrap();

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..self.vertex_count, 0..1);
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

/// Generate grid line vertices with default colors
fn generate_grid_vertices(size: f32, spacing: f32) -> Vec<PositionColorVertex> {
    generate_grid_vertices_with_colors(
        size,
        spacing,
        constants::LINE_COLOR,
        constants::X_AXIS_COLOR,
        constants::Y_AXIS_COLOR,
    )
}

/// Generate grid line vertices with custom colors
fn generate_grid_vertices_with_colors(
    size: f32,
    spacing: f32,
    line_color: [f32; 3],
    x_axis_color: [f32; 3],
    y_axis_color: [f32; 3],
) -> Vec<PositionColorVertex> {
    let mut vertices = Vec::new();
    let half_size = size;
    let num_lines = (size / spacing) as i32;

    // Lines parallel to X axis
    for i in -num_lines..=num_lines {
        let y = i as f32 * spacing;
        let color = if i == 0 { x_axis_color } else { line_color };

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
        let color = if i == 0 { y_axis_color } else { line_color };

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
