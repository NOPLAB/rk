//! Coordinate axis gizmo renderer

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::util::DeviceExt;

use crate::constants::instances;
use crate::instanced::InstanceBuffer;
use crate::pipeline::{PipelineConfig, create_camera_bind_group};
use crate::vertex::{PositionColorVertex, mat4_instance_attributes};

/// Axis instance data - passed as vertex instance
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct AxisInstance {
    pub transform: [[f32; 4]; 4],
    pub scale: f32,
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
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    instances: InstanceBuffer<AxisInstance>,
    bind_group: wgpu::BindGroup,
}

impl AxisRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) -> Self {
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
        let vertex_count = vertices.len() as u32;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Axis Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instances = InstanceBuffer::new(device, "Axis", instances::MAX_AXES);

        Self {
            pipeline,
            vertex_buffer,
            vertex_count,
            instances,
            bind_group,
        }
    }

    /// Update axis instances
    pub fn update_instances(&mut self, queue: &wgpu::Queue, instances: &[AxisInstance]) {
        self.instances.update(queue, instances);
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

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.instances.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instances.slice());
        render_pass.draw(0..self.vertex_count, 0..self.instances.count());
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
