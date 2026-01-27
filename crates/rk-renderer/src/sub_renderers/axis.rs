//! Coordinate axis gizmo renderer

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::util::DeviceExt;

use crate::constants::instances;
use crate::context::RenderContext;
use crate::instanced::InstanceBuffer;
use crate::pipeline::{PipelineConfig, create_camera_bind_group};
use crate::scene::Scene;
use crate::traits::SubRenderer;
use crate::vertex::{PositionColorVertex, mat4_instance_attributes};

/// Axis instance data - passed as vertex instance
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct AxisInstance {
    /// Transformation matrix for this axis instance.
    pub transform: [[f32; 4]; 4],
    /// Scale factor for the axis lines.
    pub scale: f32,
    /// Padding for alignment.
    pub _pad: [f32; 3],
}

impl Default for AxisInstance {
    fn default() -> Self {
        Self {
            transform: Mat4::IDENTITY.to_cols_array_2d(),
            scale: 1.0,
            _pad: [0.0; 3],
        }
    }
}

/// Axis renderer for coordinate frame visualization
pub struct AxisRenderer {
    enabled: bool,
    initialized: bool,
    pipeline: Option<wgpu::RenderPipeline>,
    vertex_buffer: Option<wgpu::Buffer>,
    vertex_count: u32,
    instances: Option<InstanceBuffer<AxisInstance>>,
    bind_group: Option<wgpu::BindGroup>,
}

impl AxisRenderer {
    /// Creates a new axis renderer (uninitialized).
    pub fn new() -> Self {
        Self {
            enabled: true,
            initialized: false,
            pipeline: None,
            vertex_buffer: None,
            vertex_count: 0,
            instances: None,
            bind_group: None,
        }
    }

    /// Initialize the axis renderer with GPU resources.
    pub fn init(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) {
        let bind_group =
            create_camera_bind_group(device, camera_bind_group_layout, camera_buffer, "Axis");

        // Instance buffer layout: Mat4 (4 x Float32x4) + scale + padding (Float32x4)
        let mat4_attrs = mat4_instance_attributes(2);
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<AxisInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                mat4_attrs[0],
                mat4_attrs[1],
                mat4_attrs[2],
                mat4_attrs[3],
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };

        let pipeline = PipelineConfig::new(
            "Axis",
            include_str!("../shaders/axis.wgsl"),
            format,
            depth_format,
            &[camera_bind_group_layout],
        )
        .with_vertex_layouts(vec![PositionColorVertex::layout(), instance_layout])
        .with_topology(wgpu::PrimitiveTopology::LineList)
        .build(device);

        // Generate axis vertices (X=red, Y=green, Z=blue)
        let vertices = generate_axis_vertices();
        self.vertex_count = vertices.len() as u32;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Axis Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instances = InstanceBuffer::new(device, "Axis", instances::MAX_AXES);

        self.pipeline = Some(pipeline);
        self.vertex_buffer = Some(vertex_buffer);
        self.instances = Some(instances);
        self.bind_group = Some(bind_group);
        self.initialized = true;
    }

    /// Update axis instances
    pub fn update_instances(&mut self, queue: &wgpu::Queue, instances: &[AxisInstance]) {
        if let Some(ref mut inst_buffer) = self.instances {
            inst_buffer.update(queue, instances);
        }
    }

    /// Add a single axis at the given transform
    pub fn set_single_axis(&mut self, queue: &wgpu::Queue, transform: Mat4, scale: f32) {
        let instance = AxisInstance {
            transform: transform.to_cols_array_2d(),
            scale,
            _pad: [0.0; 3],
        };
        self.update_instances(queue, &[instance]);
    }

    /// Renders all axis instances (standalone method for legacy API).
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if !self.initialized {
            return;
        }

        let instances = self.instances.as_ref().unwrap();
        if instances.is_empty() {
            return;
        }

        let pipeline = self.pipeline.as_ref().unwrap();
        let vertex_buffer = self.vertex_buffer.as_ref().unwrap();
        let bind_group = self.bind_group.as_ref().unwrap();

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, instances.slice());
        render_pass.draw(0..self.vertex_count, 0..instances.count());
    }
}

impl Default for AxisRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SubRenderer for AxisRenderer {
    fn name(&self) -> &str {
        "axis"
    }

    fn priority(&self) -> i32 {
        super::priorities::AXIS
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn on_init(&mut self, ctx: &RenderContext) {
        let bind_group = create_camera_bind_group(
            ctx.device(),
            ctx.camera_bind_group_layout(),
            ctx.camera_buffer(),
            "Axis",
        );

        // Instance buffer layout: Mat4 (4 x Float32x4) + scale + padding (Float32x4)
        let mat4_attrs = mat4_instance_attributes(2);
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<AxisInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                mat4_attrs[0],
                mat4_attrs[1],
                mat4_attrs[2],
                mat4_attrs[3],
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };

        let pipeline = PipelineConfig::new(
            "Axis",
            include_str!("../shaders/axis.wgsl"),
            ctx.surface_format(),
            ctx.depth_format(),
            &[ctx.camera_bind_group_layout()],
        )
        .with_vertex_layouts(vec![PositionColorVertex::layout(), instance_layout])
        .with_topology(wgpu::PrimitiveTopology::LineList)
        .build(ctx.device());

        // Generate axis vertices (X=red, Y=green, Z=blue)
        let vertices = generate_axis_vertices();
        self.vertex_count = vertices.len() as u32;

        let vertex_buffer = ctx.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Axis Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instances = InstanceBuffer::new(ctx.device(), "Axis", instances::MAX_AXES);

        self.pipeline = Some(pipeline);
        self.vertex_buffer = Some(vertex_buffer);
        self.instances = Some(instances);
        self.bind_group = Some(bind_group);
        self.initialized = true;
    }

    fn on_resize(&mut self, _ctx: &RenderContext, _width: u32, _height: u32) {
        // Axis renderer doesn't need to respond to resize
    }

    fn prepare(&mut self, _ctx: &RenderContext, _scene: &Scene) {
        // Axis data is updated externally via update_instances
    }

    fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, _scene: &Scene) {
        AxisRenderer::render(self, pass);
    }
}

fn generate_axis_vertices() -> Vec<PositionColorVertex> {
    vec![
        // X axis (red)
        PositionColorVertex {
            position: [0.0, 0.0, 0.0],
            color: [1.0, 0.0, 0.0],
        },
        PositionColorVertex {
            position: [1.0, 0.0, 0.0],
            color: [1.0, 0.0, 0.0],
        },
        // Y axis (green)
        PositionColorVertex {
            position: [0.0, 0.0, 0.0],
            color: [0.0, 1.0, 0.0],
        },
        PositionColorVertex {
            position: [0.0, 1.0, 0.0],
            color: [0.0, 1.0, 0.0],
        },
        // Z axis (blue)
        PositionColorVertex {
            position: [0.0, 0.0, 0.0],
            color: [0.0, 0.0, 1.0],
        },
        PositionColorVertex {
            position: [0.0, 0.0, 1.0],
            color: [0.0, 0.0, 1.0],
        },
    ]
}
