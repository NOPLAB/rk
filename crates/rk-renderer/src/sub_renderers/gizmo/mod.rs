//! Transform gizmo renderer
//!
//! This module provides a 3D transform gizmo for manipulating objects
//! in the viewport. Supports translation and rotation modes.

mod collision;
mod geometry;

pub use collision::{ray_cylinder_intersection, ray_ring_intersection};
pub use geometry::GizmoVertex;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::constants::gizmo as constants;
use crate::constants::viewport::SAMPLE_COUNT;
use geometry::{generate_rotation_gizmo, generate_scale_gizmo, generate_translation_gizmo};

/// Gizmo mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GizmoMode {
    #[default]
    Translate,
    Rotate,
    Scale,
}

/// Which axis is being manipulated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GizmoAxis {
    #[default]
    None,
    X,
    Y,
    Z,
}

impl GizmoAxis {
    pub fn to_index(&self) -> i32 {
        match self {
            GizmoAxis::None => -1,
            GizmoAxis::X => 0,
            GizmoAxis::Y => 1,
            GizmoAxis::Z => 2,
        }
    }

    pub fn direction(&self) -> Vec3 {
        match self {
            GizmoAxis::None => Vec3::ZERO,
            GizmoAxis::X => Vec3::X,
            GizmoAxis::Y => Vec3::Y,
            GizmoAxis::Z => Vec3::Z,
        }
    }
}

/// Gizmo instance data
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GizmoInstance {
    pub transform: [[f32; 4]; 4],
    pub scale: f32,
    pub highlighted_axis: f32, // -1=none, 0=X, 1=Y, 2=Z
    pub _pad: [f32; 2],
}

impl Default for GizmoInstance {
    fn default() -> Self {
        Self {
            transform: Mat4::IDENTITY.to_cols_array_2d(),
            scale: 1.0,
            highlighted_axis: -1.0,
            _pad: [0.0; 2],
        }
    }
}

/// Gizmo renderer
pub struct GizmoRenderer {
    pipeline: wgpu::RenderPipeline,
    // Translation gizmo buffers
    translate_vertex_buffer: wgpu::Buffer,
    translate_index_buffer: wgpu::Buffer,
    translate_index_count: u32,
    // Rotation gizmo buffers
    rotate_vertex_buffer: wgpu::Buffer,
    rotate_index_buffer: wgpu::Buffer,
    rotate_index_count: u32,
    // Scale gizmo buffers
    scale_vertex_buffer: wgpu::Buffer,
    scale_index_buffer: wgpu::Buffer,
    scale_index_count: u32,
    // Shared buffers
    instance_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pub visible: bool,
    pub mode: GizmoMode,
    pub highlighted_axis: GizmoAxis,
    instance: GizmoInstance,
}

impl GizmoRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Gizmo Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/gizmo.wgsl").into()),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Gizmo Camera Bind Group"),
            layout: camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Gizmo Pipeline Layout"),
            bind_group_layouts: &[camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Gizmo Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    // Vertex buffer
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GizmoVertex>() as u64,
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
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 28,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Uint32,
                            },
                        ],
                    },
                    // Instance buffer
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GizmoInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 3,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 16,
                                shader_location: 4,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 32,
                                shader_location: 5,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 48,
                                shader_location: 6,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: 64,
                                shader_location: 7,
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
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: false, // Gizmo always on top
                depth_compare: wgpu::CompareFunction::Always,
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

        // Translation gizmo geometry
        let (translate_vertices, translate_indices) = generate_translation_gizmo();
        let translate_index_count = translate_indices.len() as u32;

        let translate_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Gizmo Translate Vertex Buffer"),
                contents: bytemuck::cast_slice(&translate_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let translate_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Translate Index Buffer"),
            contents: bytemuck::cast_slice(&translate_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Rotation gizmo geometry
        let (rotate_vertices, rotate_indices) = generate_rotation_gizmo();
        let rotate_index_count = rotate_indices.len() as u32;

        let rotate_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Rotate Vertex Buffer"),
            contents: bytemuck::cast_slice(&rotate_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let rotate_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Rotate Index Buffer"),
            contents: bytemuck::cast_slice(&rotate_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Scale gizmo geometry
        let (scale_vertices, scale_indices) = generate_scale_gizmo();
        let scale_index_count = scale_indices.len() as u32;

        let scale_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Scale Vertex Buffer"),
            contents: bytemuck::cast_slice(&scale_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let scale_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Scale Index Buffer"),
            contents: bytemuck::cast_slice(&scale_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Gizmo Instance Buffer"),
            contents: bytemuck::cast_slice(&[GizmoInstance::default()]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            pipeline,
            translate_vertex_buffer,
            translate_index_buffer,
            translate_index_count,
            rotate_vertex_buffer,
            rotate_index_buffer,
            rotate_index_count,
            scale_vertex_buffer,
            scale_index_buffer,
            scale_index_count,
            instance_buffer,
            bind_group,
            visible: false,
            mode: GizmoMode::Translate,
            highlighted_axis: GizmoAxis::None,
            instance: GizmoInstance::default(),
        }
    }

    /// Set gizmo position and scale
    pub fn set_transform(&mut self, queue: &wgpu::Queue, position: Vec3, scale: f32) {
        self.instance.transform = Mat4::from_translation(position).to_cols_array_2d();
        self.instance.scale = scale;
        self.update_buffer(queue);
    }

    /// Set highlighted axis
    pub fn set_highlighted(&mut self, queue: &wgpu::Queue, axis: GizmoAxis) {
        self.highlighted_axis = axis;
        self.instance.highlighted_axis = axis.to_index() as f32;
        self.update_buffer(queue);
    }

    fn update_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&[self.instance]),
        );
    }

    /// Show the gizmo at the given position
    pub fn show(&mut self, queue: &wgpu::Queue, position: Vec3, scale: f32) {
        self.visible = true;
        self.set_transform(queue, position, scale);
    }

    /// Hide the gizmo
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Set gizmo mode
    pub fn set_mode(&mut self, mode: GizmoMode) {
        self.mode = mode;
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if !self.visible {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        // Select buffers based on mode
        match self.mode {
            GizmoMode::Translate => {
                render_pass.set_vertex_buffer(0, self.translate_vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
                render_pass.set_index_buffer(
                    self.translate_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.draw_indexed(0..self.translate_index_count, 0, 0..1);
            }
            GizmoMode::Rotate => {
                render_pass.set_vertex_buffer(0, self.rotate_vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
                render_pass.set_index_buffer(
                    self.rotate_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                render_pass.draw_indexed(0..self.rotate_index_count, 0, 0..1);
            }
            GizmoMode::Scale => {
                render_pass.set_vertex_buffer(0, self.scale_vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.scale_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.scale_index_count, 0, 0..1);
            }
        }
    }

    /// Check if a ray intersects with a gizmo axis handle.
    /// Returns the closest intersecting axis.
    pub fn hit_test(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        gizmo_pos: Vec3,
        scale: f32,
    ) -> GizmoAxis {
        match self.mode {
            GizmoMode::Translate => self.hit_test_translate(ray_origin, ray_dir, gizmo_pos, scale),
            GizmoMode::Rotate => self.hit_test_rotate(ray_origin, ray_dir, gizmo_pos, scale),
            GizmoMode::Scale => self.hit_test_scale(ray_origin, ray_dir, gizmo_pos, scale),
        }
    }

    fn hit_test_translate(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        gizmo_pos: Vec3,
        scale: f32,
    ) -> GizmoAxis {
        let handle_radius = constants::HIT_RADIUS_MULTIPLIER * scale;
        let handle_length = constants::ARROW_LENGTH * scale;

        let mut closest_axis = GizmoAxis::None;
        let mut closest_dist = f32::MAX;

        for (axis, dir) in [
            (GizmoAxis::X, Vec3::X),
            (GizmoAxis::Y, Vec3::Y),
            (GizmoAxis::Z, Vec3::Z),
        ] {
            // Test intersection with cylinder along axis
            let axis_start = gizmo_pos;
            let axis_end = gizmo_pos + dir * handle_length;

            if let Some(dist) = collision::ray_cylinder_intersection(
                ray_origin,
                ray_dir,
                axis_start,
                axis_end,
                handle_radius,
            ) && dist < closest_dist
            {
                closest_dist = dist;
                closest_axis = axis;
            }
        }

        closest_axis
    }

    fn hit_test_rotate(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        gizmo_pos: Vec3,
        scale: f32,
    ) -> GizmoAxis {
        let ring_radius = constants::RING_RADIUS * scale;
        let hit_thickness = constants::RING_HIT_THICKNESS * scale;

        let mut closest_axis = GizmoAxis::None;
        let mut closest_dist = f32::MAX;

        for (axis, normal) in [
            (GizmoAxis::X, Vec3::X),
            (GizmoAxis::Y, Vec3::Y),
            (GizmoAxis::Z, Vec3::Z),
        ] {
            if let Some(dist) = collision::ray_ring_intersection(
                ray_origin,
                ray_dir,
                gizmo_pos,
                normal,
                ring_radius,
                hit_thickness,
            ) && dist < closest_dist
            {
                closest_dist = dist;
                closest_axis = axis;
            }
        }

        closest_axis
    }

    fn hit_test_scale(
        &self,
        ray_origin: Vec3,
        ray_dir: Vec3,
        gizmo_pos: Vec3,
        scale: f32,
    ) -> GizmoAxis {
        let axis_length = constants::SCALE_AXIS_LENGTH * scale;
        let hit_size = constants::SCALE_HIT_SIZE * scale;

        let mut closest_axis = GizmoAxis::None;
        let mut closest_dist = f32::MAX;

        for (axis, dir) in [
            (GizmoAxis::X, Vec3::X),
            (GizmoAxis::Y, Vec3::Y),
            (GizmoAxis::Z, Vec3::Z),
        ] {
            // Test intersection with cube at end of axis (using cylinder approximation)
            let cube_center = gizmo_pos + dir * axis_length;

            // Simple sphere-based hit test for the cube
            if let Some(dist) = ray_sphere_intersection(ray_origin, ray_dir, cube_center, hit_size)
                && dist < closest_dist
            {
                closest_dist = dist;
                closest_axis = axis;
            }
        }

        closest_axis
    }
}

/// Ray-sphere intersection helper for scale gizmo cube hit test
fn ray_sphere_intersection(
    ray_origin: Vec3,
    ray_dir: Vec3,
    sphere_center: Vec3,
    radius: f32,
) -> Option<f32> {
    let oc = ray_origin - sphere_center;
    let a = ray_dir.dot(ray_dir);
    let b = 2.0 * oc.dot(ray_dir);
    let c = oc.dot(oc) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return None;
    }

    let t = (-b - discriminant.sqrt()) / (2.0 * a);
    if t > 0.0 { Some(t) } else { None }
}
