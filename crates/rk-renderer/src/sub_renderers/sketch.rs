//! Sketch sub-renderer for 2D sketch visualization.
//!
//! Renders sketch entities (points, lines, circles, arcs) on a sketch plane
//! in 3D space. Supports selection highlighting and constraint visualization.

use glam::{Mat4, Vec2, Vec3, Vec4};
use std::collections::HashMap;
use uuid::Uuid;
use wgpu::util::DeviceExt;

use crate::context::RenderContext;
use crate::pipeline::PipelineConfig;
use crate::scene::Scene;
use crate::traits::SubRenderer;

/// Vertex for sketch rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SketchVertex {
    /// Position in sketch space (z = 0 for 2D sketches).
    pub position: [f32; 3],
    /// Vertex color (RGBA).
    pub color: [f32; 4],
    /// Flags: bit 0 = selected, bit 1 = hovered, bit 2 = construction, bit 3 = constrained.
    pub flags: u32,
}

impl SketchVertex {
    /// Vertex attributes for the shader.
    pub const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &[
        wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x3,
        },
        wgpu::VertexAttribute {
            offset: std::mem::size_of::<[f32; 3]>() as u64,
            shader_location: 1,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: (std::mem::size_of::<[f32; 3]>() + std::mem::size_of::<[f32; 4]>()) as u64,
            shader_location: 2,
            format: wgpu::VertexFormat::Uint32,
        },
    ];

    /// Returns the vertex buffer layout.
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }

    /// Create a new vertex.
    pub fn new(position: Vec3, color: Vec4, flags: u32) -> Self {
        Self {
            position: position.to_array(),
            color: color.to_array(),
            flags,
        }
    }
}

/// Vertex flag constants.
pub mod flags {
    /// Entity is selected.
    pub const SELECTED: u32 = 1;
    /// Entity is hovered.
    pub const HOVERED: u32 = 2;
    /// Entity is construction geometry.
    pub const CONSTRUCTION: u32 = 4;
    /// Entity is fully constrained.
    pub const CONSTRAINED: u32 = 8;
}

/// Uniform data for sketch rendering.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SketchUniform {
    /// Transform from sketch space to world space.
    transform: [[f32; 4]; 4],
    /// Sketch plane visualization color.
    plane_color: [f32; 4],
}

/// Data for a constraint icon to be rendered as egui overlay.
#[derive(Debug, Clone)]
pub struct ConstraintIconData {
    /// Constraint ID.
    pub id: Uuid,
    /// Position in sketch coordinate space.
    pub position: Vec2,
    /// Constraint type name (e.g., "Coincident", "Horizontal").
    pub constraint_type: String,
    /// Dimensional value (if applicable).
    pub value: Option<f32>,
    /// Dimension line data for dimensional constraints (Distance, Angle, etc.).
    pub dimension_line: Option<DimensionLine>,
}

/// Dimension line data for CAD-style dimension display.
#[derive(Debug, Clone)]
pub struct DimensionLine {
    /// Start point of the dimension line (in sketch coordinates).
    pub start: Vec2,
    /// End point of the dimension line (in sketch coordinates).
    pub end: Vec2,
    /// Extension line start point (optional, for offset dimension lines).
    pub extension_start: Option<Vec2>,
    /// Extension line end point (optional, for offset dimension lines).
    pub extension_end: Option<Vec2>,
    /// Arc data for angle constraints.
    pub arc_data: Option<ArcData>,
}

/// Arc data for angle dimension display.
#[derive(Debug, Clone)]
pub struct ArcData {
    /// Center of the arc (in sketch coordinates).
    pub center: Vec2,
    /// Radius of the arc.
    pub radius: f32,
    /// Start angle in radians.
    pub start_angle: f32,
    /// End angle in radians.
    pub end_angle: f32,
}

/// Data for a single sketch to be rendered.
#[derive(Debug, Clone)]
pub struct SketchRenderData {
    /// Sketch ID.
    pub id: Uuid,
    /// Transform from sketch space to world space.
    pub transform: Mat4,
    /// Line vertices (pairs of vertices for each line segment).
    pub line_vertices: Vec<SketchVertex>,
    /// Point vertices.
    pub point_vertices: Vec<SketchVertex>,
    /// Whether this sketch is currently being edited.
    pub is_active: bool,
    /// Constraint icons to be rendered as egui overlay.
    pub constraint_icons: Vec<ConstraintIconData>,
}

impl Default for SketchRenderData {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            transform: Mat4::IDENTITY,
            line_vertices: Vec::new(),
            point_vertices: Vec::new(),
            is_active: false,
            constraint_icons: Vec::new(),
        }
    }
}

impl SketchRenderData {
    /// Create new sketch render data.
    pub fn new(id: Uuid, transform: Mat4) -> Self {
        Self {
            id,
            transform,
            line_vertices: Vec::new(),
            point_vertices: Vec::new(),
            is_active: false,
            constraint_icons: Vec::new(),
        }
    }

    /// Half-size of point quad in sketch units
    const POINT_HALF_SIZE: f32 = 0.008;

    /// Add a point to the sketch (as a quad for visibility).
    pub fn add_point(&mut self, position: Vec2, color: Vec4, flags: u32) {
        let hs = Self::POINT_HALF_SIZE;
        let p = position;

        // Create two triangles forming a quad (CCW winding)
        // Triangle 1: bottom-left, bottom-right, top-right
        self.point_vertices.push(SketchVertex::new(
            Vec3::new(p.x - hs, p.y - hs, 0.0),
            color,
            flags,
        ));
        self.point_vertices.push(SketchVertex::new(
            Vec3::new(p.x + hs, p.y - hs, 0.0),
            color,
            flags,
        ));
        self.point_vertices.push(SketchVertex::new(
            Vec3::new(p.x + hs, p.y + hs, 0.0),
            color,
            flags,
        ));

        // Triangle 2: bottom-left, top-right, top-left
        self.point_vertices.push(SketchVertex::new(
            Vec3::new(p.x - hs, p.y - hs, 0.0),
            color,
            flags,
        ));
        self.point_vertices.push(SketchVertex::new(
            Vec3::new(p.x + hs, p.y + hs, 0.0),
            color,
            flags,
        ));
        self.point_vertices.push(SketchVertex::new(
            Vec3::new(p.x - hs, p.y + hs, 0.0),
            color,
            flags,
        ));
    }

    /// Add a line segment to the sketch.
    pub fn add_line(&mut self, start: Vec2, end: Vec2, color: Vec4, flags: u32) {
        self.line_vertices.push(SketchVertex::new(
            Vec3::new(start.x, start.y, 0.0),
            color,
            flags,
        ));
        self.line_vertices.push(SketchVertex::new(
            Vec3::new(end.x, end.y, 0.0),
            color,
            flags,
        ));
    }

    /// Add a circle as line segments.
    pub fn add_circle(
        &mut self,
        center: Vec2,
        radius: f32,
        color: Vec4,
        flags: u32,
        segments: u32,
    ) {
        let step = std::f32::consts::TAU / segments as f32;
        for i in 0..segments {
            let angle1 = i as f32 * step;
            let angle2 = (i + 1) as f32 * step;
            let p1 = center + Vec2::new(angle1.cos() * radius, angle1.sin() * radius);
            let p2 = center + Vec2::new(angle2.cos() * radius, angle2.sin() * radius);
            self.add_line(p1, p2, color, flags);
        }
    }

    /// Add an arc as line segments.
    #[allow(clippy::too_many_arguments)]
    pub fn add_arc(
        &mut self,
        center: Vec2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        color: Vec4,
        flags: u32,
        segments: u32,
    ) {
        let arc_angle = end_angle - start_angle;
        let step = arc_angle / segments as f32;
        for i in 0..segments {
            let angle1 = start_angle + i as f32 * step;
            let angle2 = start_angle + (i + 1) as f32 * step;
            let p1 = center + Vec2::new(angle1.cos() * radius, angle1.sin() * radius);
            let p2 = center + Vec2::new(angle2.cos() * radius, angle2.sin() * radius);
            self.add_line(p1, p2, color, flags);
        }
    }

    /// Clear all geometry.
    pub fn clear(&mut self) {
        self.line_vertices.clear();
        self.point_vertices.clear();
    }
}

/// Sketch sub-renderer.
pub struct SketchRenderer {
    enabled: bool,
    initialized: bool,
    line_pipeline: Option<wgpu::RenderPipeline>,
    point_pipeline: Option<wgpu::RenderPipeline>,
    camera_bind_group: Option<wgpu::BindGroup>,
    sketch_bind_group_layout: Option<wgpu::BindGroupLayout>,

    /// Per-sketch GPU resources.
    sketch_resources: HashMap<Uuid, SketchGpuResources>,

    /// Sketch data to render (updated each frame).
    pending_sketches: Vec<SketchRenderData>,
}

#[allow(dead_code)]
struct SketchGpuResources {
    line_buffer: wgpu::Buffer,
    point_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    line_count: u32,
    point_count: u32,
}

impl Default for SketchRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SketchRenderer {
    /// Create a new sketch renderer (uninitialized).
    pub fn new() -> Self {
        Self {
            enabled: true,
            initialized: false,
            line_pipeline: None,
            point_pipeline: None,
            camera_bind_group: None,
            sketch_bind_group_layout: None,
            sketch_resources: HashMap::new(),
            pending_sketches: Vec::new(),
        }
    }

    /// Initialize the sketch renderer with GPU resources.
    ///
    /// This follows the same pattern as other sub-renderers (GridRenderer, etc.)
    pub fn init(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) {
        // Create sketch uniform bind group layout
        let sketch_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Sketch Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Create line pipeline
        let line_pipeline = PipelineConfig::new(
            "Sketch Lines",
            include_str!("../shaders/sketch.wgsl"),
            format,
            depth_format,
            &[camera_bind_group_layout, &sketch_bind_group_layout],
        )
        .with_vertex_layouts(vec![SketchVertex::layout()])
        .with_topology(wgpu::PrimitiveTopology::LineList)
        .with_blend(wgpu::BlendState::ALPHA_BLENDING)
        .with_depth_write(false)
        .build(device);

        // Create point pipeline (using triangles to draw square points)
        let point_pipeline = PipelineConfig::new(
            "Sketch Points",
            include_str!("../shaders/sketch.wgsl"),
            format,
            depth_format,
            &[camera_bind_group_layout, &sketch_bind_group_layout],
        )
        .with_vertex_layouts(vec![SketchVertex::layout()])
        .with_topology(wgpu::PrimitiveTopology::TriangleList)
        .with_blend(wgpu::BlendState::ALPHA_BLENDING)
        .with_depth_write(false)
        .with_entry_point("vs_point", "fs_point")
        .build(device);

        // Create camera bind group
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sketch Camera Bind Group"),
            layout: camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        self.line_pipeline = Some(line_pipeline);
        self.point_pipeline = Some(point_pipeline);
        self.camera_bind_group = Some(camera_bind_group);
        self.sketch_bind_group_layout = Some(sketch_bind_group_layout);
        self.initialized = true;
    }

    /// Set sketch data to render.
    pub fn set_sketches(&mut self, sketches: Vec<SketchRenderData>) {
        self.pending_sketches = sketches;
    }

    /// Add a single sketch to render.
    pub fn add_sketch(&mut self, sketch: SketchRenderData) {
        self.pending_sketches.push(sketch);
    }

    /// Clear all sketches.
    pub fn clear_sketches(&mut self) {
        self.pending_sketches.clear();
    }

    /// Prepare GPU resources for rendering (standalone version without RenderContext).
    pub fn prepare_with_device(&mut self, device: &wgpu::Device) {
        if !self.initialized {
            return;
        }

        let layout = self.sketch_bind_group_layout.as_ref().unwrap();

        // Update GPU resources for each sketch
        for sketch_data in &self.pending_sketches {
            let uniform = SketchUniform {
                transform: sketch_data.transform.to_cols_array_2d(),
                plane_color: [0.5, 0.5, 0.5, 0.2],
            };

            let line_count = sketch_data.line_vertices.len() as u32;
            let point_count = sketch_data.point_vertices.len() as u32;

            // Create buffers
            let line_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sketch Line Buffer"),
                contents: if sketch_data.line_vertices.is_empty() {
                    &[0u8; std::mem::size_of::<SketchVertex>()]
                } else {
                    bytemuck::cast_slice(&sketch_data.line_vertices)
                },
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

            let point_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sketch Point Buffer"),
                contents: if sketch_data.point_vertices.is_empty() {
                    &[0u8; std::mem::size_of::<SketchVertex>()]
                } else {
                    bytemuck::cast_slice(&sketch_data.point_vertices)
                },
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

            let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sketch Uniform Buffer"),
                contents: bytemuck::bytes_of(&uniform),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Sketch Bind Group"),
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });

            self.sketch_resources.insert(
                sketch_data.id,
                SketchGpuResources {
                    line_buffer,
                    point_buffer,
                    uniform_buffer,
                    bind_group,
                    line_count,
                    point_count,
                },
            );
        }

        // Remove resources for sketches no longer being rendered
        let active_ids: std::collections::HashSet<Uuid> =
            self.pending_sketches.iter().map(|s| s.id).collect();
        self.sketch_resources
            .retain(|id, _| active_ids.contains(id));
    }

    /// Render sketches (standalone version without Scene).
    pub fn render_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if !self.initialized || self.pending_sketches.is_empty() {
            return;
        }

        let line_pipeline = self.line_pipeline.as_ref().unwrap();
        let point_pipeline = self.point_pipeline.as_ref().unwrap();
        let camera_bind_group = self.camera_bind_group.as_ref().unwrap();

        // Render lines for each sketch
        pass.set_pipeline(line_pipeline);
        pass.set_bind_group(0, camera_bind_group, &[]);

        for sketch_data in &self.pending_sketches {
            if let Some(resources) = self.sketch_resources.get(&sketch_data.id)
                && resources.line_count > 0
            {
                pass.set_bind_group(1, &resources.bind_group, &[]);
                pass.set_vertex_buffer(0, resources.line_buffer.slice(..));
                pass.draw(0..resources.line_count, 0..1);
            }
        }

        // Render points for each sketch
        pass.set_pipeline(point_pipeline);

        for sketch_data in &self.pending_sketches {
            if let Some(resources) = self.sketch_resources.get(&sketch_data.id)
                && resources.point_count > 0
            {
                pass.set_bind_group(1, &resources.bind_group, &[]);
                pass.set_vertex_buffer(0, resources.point_buffer.slice(..));
                pass.draw(0..resources.point_count, 0..1);
            }
        }
    }
}

impl SubRenderer for SketchRenderer {
    fn name(&self) -> &str {
        "sketch"
    }

    fn priority(&self) -> i32 {
        super::priorities::SKETCH
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
        // Sketch renderer doesn't need to respond to resize
    }

    fn prepare(&mut self, ctx: &RenderContext, _scene: &Scene) {
        self.prepare_with_device(ctx.device());
    }

    fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, _scene: &Scene) {
        self.render_pass(pass);
    }
}
