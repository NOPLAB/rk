//! Collision shape visualization renderer

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::constants::{collision as constants, instances};
use crate::pipeline::{PipelineConfig, create_camera_bind_group};

/// Vertex with position and normal for collision geometry
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CollisionVertex {
    /// Vertex position
    pub position: [f32; 3],
    /// Vertex normal
    pub normal: [f32; 3],
}

impl CollisionVertex {
    /// Create a new collision vertex
    pub fn new(position: Vec3, normal: Vec3) -> Self {
        Self {
            position: position.to_array(),
            normal: normal.to_array(),
        }
    }

    /// Get the vertex buffer layout for collision vertices
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
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
        }
    }
}

/// Collision instance data for GPU instancing
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CollisionInstance {
    /// Model transformation matrix (4x4)
    pub model: [[f32; 4]; 4],
    /// Instance color (RGBA)
    pub color: [f32; 4],
}

impl CollisionInstance {
    /// Create a new collision instance
    pub fn new(transform: Mat4, color: [f32; 4]) -> Self {
        Self {
            model: transform.to_cols_array_2d(),
            color,
        }
    }

    /// Get the instance buffer layout
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // model matrix (4 columns)
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // color
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

impl Default for CollisionInstance {
    fn default() -> Self {
        Self {
            model: Mat4::IDENTITY.to_cols_array_2d(),
            color: constants::DEFAULT_COLOR,
        }
    }
}

/// Geometry type for collision shapes
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum CollisionGeometry {
    /// Box collision shape
    Box,
    /// Sphere collision shape
    Sphere,
    /// Cylinder collision shape
    Cylinder,
    /// Capsule collision shape
    Capsule,
}

/// Collision renderer for visualizing collision shapes
pub struct CollisionRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,

    // Box geometry
    box_vertex_buffer: wgpu::Buffer,
    box_index_buffer: wgpu::Buffer,
    box_index_count: u32,
    box_instance_buffer: wgpu::Buffer,
    box_instances: Vec<CollisionInstance>,

    // Sphere geometry
    sphere_vertex_buffer: wgpu::Buffer,
    sphere_index_buffer: wgpu::Buffer,
    sphere_index_count: u32,
    sphere_instance_buffer: wgpu::Buffer,
    sphere_instances: Vec<CollisionInstance>,

    // Cylinder geometry
    cylinder_vertex_buffer: wgpu::Buffer,
    cylinder_index_buffer: wgpu::Buffer,
    cylinder_index_count: u32,
    cylinder_instance_buffer: wgpu::Buffer,
    cylinder_instances: Vec<CollisionInstance>,

    // Capsule geometry
    capsule_vertex_buffer: wgpu::Buffer,
    capsule_index_buffer: wgpu::Buffer,
    capsule_index_count: u32,
    capsule_instance_buffer: wgpu::Buffer,
    capsule_instances: Vec<CollisionInstance>,

    visible: bool,
}

impl CollisionRenderer {
    /// Creates a new collision renderer.
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) -> Self {
        let bind_group =
            create_camera_bind_group(device, camera_bind_group_layout, camera_buffer, "Collision");

        // Create pipeline with alpha blending (default in PipelineConfig)
        let pipeline = PipelineConfig::new(
            "Collision",
            include_str!("../shaders/collision.wgsl"),
            format,
            depth_format,
            &[camera_bind_group_layout],
        )
        .with_vertex_layouts(vec![CollisionVertex::layout(), CollisionInstance::layout()])
        .with_cull_mode(None) // Double-sided
        .build(device);

        // Generate geometries
        let (box_vertices, box_indices) = generate_box();
        let (sphere_vertices, sphere_indices) =
            generate_sphere(constants::SEGMENTS, constants::RINGS);
        let (cylinder_vertices, cylinder_indices) = generate_cylinder(constants::SEGMENTS);
        let (capsule_vertices, capsule_indices) =
            generate_capsule(constants::SEGMENTS, constants::RINGS / 2);

        // Create buffers
        let box_vertex_buffer = create_vertex_buffer(device, "Box", &box_vertices);
        let box_index_buffer = create_index_buffer(device, "Box", &box_indices);
        let box_instance_buffer = create_instance_buffer(device, "Box");

        let sphere_vertex_buffer = create_vertex_buffer(device, "Sphere", &sphere_vertices);
        let sphere_index_buffer = create_index_buffer(device, "Sphere", &sphere_indices);
        let sphere_instance_buffer = create_instance_buffer(device, "Sphere");

        let cylinder_vertex_buffer = create_vertex_buffer(device, "Cylinder", &cylinder_vertices);
        let cylinder_index_buffer = create_index_buffer(device, "Cylinder", &cylinder_indices);
        let cylinder_instance_buffer = create_instance_buffer(device, "Cylinder");

        let capsule_vertex_buffer = create_vertex_buffer(device, "Capsule", &capsule_vertices);
        let capsule_index_buffer = create_index_buffer(device, "Capsule", &capsule_indices);
        let capsule_instance_buffer = create_instance_buffer(device, "Capsule");

        Self {
            pipeline,
            bind_group,
            box_vertex_buffer,
            box_index_buffer,
            box_index_count: box_indices.len() as u32,
            box_instance_buffer,
            box_instances: Vec::new(),
            sphere_vertex_buffer,
            sphere_index_buffer,
            sphere_index_count: sphere_indices.len() as u32,
            sphere_instance_buffer,
            sphere_instances: Vec::new(),
            cylinder_vertex_buffer,
            cylinder_index_buffer,
            cylinder_index_count: cylinder_indices.len() as u32,
            cylinder_instance_buffer,
            cylinder_instances: Vec::new(),
            capsule_vertex_buffer,
            capsule_index_buffer,
            capsule_index_count: capsule_indices.len() as u32,
            capsule_instance_buffer,
            capsule_instances: Vec::new(),
            visible: true,
        }
    }

    /// Set visibility
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Clear all instances
    pub fn clear(&mut self) {
        self.box_instances.clear();
        self.sphere_instances.clear();
        self.cylinder_instances.clear();
        self.capsule_instances.clear();
    }

    /// Add a box collision instance
    pub fn add_box(&mut self, transform: Mat4, size: [f32; 3], color: [f32; 4]) {
        let scale = Mat4::from_scale(Vec3::from_array(size));
        let instance = CollisionInstance::new(transform * scale, color);
        self.box_instances.push(instance);
    }

    /// Add a sphere collision instance
    pub fn add_sphere(&mut self, transform: Mat4, radius: f32, color: [f32; 4]) {
        let scale = Mat4::from_scale(Vec3::splat(radius));
        let instance = CollisionInstance::new(transform * scale, color);
        self.sphere_instances.push(instance);
    }

    /// Add a cylinder collision instance
    pub fn add_cylinder(&mut self, transform: Mat4, radius: f32, length: f32, color: [f32; 4]) {
        // Cylinder is along Z axis, scale appropriately
        let scale = Mat4::from_scale(Vec3::new(radius, radius, length));
        let instance = CollisionInstance::new(transform * scale, color);
        self.cylinder_instances.push(instance);
    }

    /// Add a capsule collision instance
    pub fn add_capsule(&mut self, transform: Mat4, radius: f32, length: f32, color: [f32; 4]) {
        // Capsule is along Z axis
        let scale = Mat4::from_scale(Vec3::new(radius, radius, length + 2.0 * radius));
        let instance = CollisionInstance::new(transform * scale, color);
        self.capsule_instances.push(instance);
    }

    /// Upload instances to GPU
    pub fn upload(&self, queue: &wgpu::Queue) {
        if !self.box_instances.is_empty() {
            queue.write_buffer(
                &self.box_instance_buffer,
                0,
                bytemuck::cast_slice(&self.box_instances),
            );
        }
        if !self.sphere_instances.is_empty() {
            queue.write_buffer(
                &self.sphere_instance_buffer,
                0,
                bytemuck::cast_slice(&self.sphere_instances),
            );
        }
        if !self.cylinder_instances.is_empty() {
            queue.write_buffer(
                &self.cylinder_instance_buffer,
                0,
                bytemuck::cast_slice(&self.cylinder_instances),
            );
        }
        if !self.capsule_instances.is_empty() {
            queue.write_buffer(
                &self.capsule_instance_buffer,
                0,
                bytemuck::cast_slice(&self.capsule_instances),
            );
        }
    }

    /// Render all collision instances
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if !self.visible {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        // Render boxes
        if !self.box_instances.is_empty() {
            render_pass.set_vertex_buffer(0, self.box_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.box_instance_buffer.slice(..));
            render_pass
                .set_index_buffer(self.box_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(
                0..self.box_index_count,
                0,
                0..self.box_instances.len() as u32,
            );
        }

        // Render spheres
        if !self.sphere_instances.is_empty() {
            render_pass.set_vertex_buffer(0, self.sphere_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.sphere_instance_buffer.slice(..));
            render_pass.set_index_buffer(
                self.sphere_index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(
                0..self.sphere_index_count,
                0,
                0..self.sphere_instances.len() as u32,
            );
        }

        // Render cylinders
        if !self.cylinder_instances.is_empty() {
            render_pass.set_vertex_buffer(0, self.cylinder_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.cylinder_instance_buffer.slice(..));
            render_pass.set_index_buffer(
                self.cylinder_index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(
                0..self.cylinder_index_count,
                0,
                0..self.cylinder_instances.len() as u32,
            );
        }

        // Render capsules
        if !self.capsule_instances.is_empty() {
            render_pass.set_vertex_buffer(0, self.capsule_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.capsule_instance_buffer.slice(..));
            render_pass.set_index_buffer(
                self.capsule_index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(
                0..self.capsule_index_count,
                0,
                0..self.capsule_instances.len() as u32,
            );
        }
    }
}

fn create_vertex_buffer(
    device: &wgpu::Device,
    name: &str,
    vertices: &[CollisionVertex],
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("Collision {} Vertex Buffer", name)),
        contents: bytemuck::cast_slice(vertices),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

fn create_index_buffer(device: &wgpu::Device, name: &str, indices: &[u32]) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("Collision {} Index Buffer", name)),
        contents: bytemuck::cast_slice(indices),
        usage: wgpu::BufferUsages::INDEX,
    })
}

fn create_instance_buffer(device: &wgpu::Device, name: &str) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("Collision {} Instance Buffer", name)),
        size: (std::mem::size_of::<CollisionInstance>() * instances::MAX_COLLISIONS as usize)
            as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

/// Generate a unit box (centered at origin, size 1x1x1)
fn generate_box() -> (Vec<CollisionVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // 6 faces, each with 4 vertices
    let faces = [
        // +X face
        (
            Vec3::X,
            [
                Vec3::new(0.5, -0.5, -0.5),
                Vec3::new(0.5, 0.5, -0.5),
                Vec3::new(0.5, 0.5, 0.5),
                Vec3::new(0.5, -0.5, 0.5),
            ],
        ),
        // -X face
        (
            Vec3::NEG_X,
            [
                Vec3::new(-0.5, -0.5, 0.5),
                Vec3::new(-0.5, 0.5, 0.5),
                Vec3::new(-0.5, 0.5, -0.5),
                Vec3::new(-0.5, -0.5, -0.5),
            ],
        ),
        // +Y face
        (
            Vec3::Y,
            [
                Vec3::new(-0.5, 0.5, -0.5),
                Vec3::new(-0.5, 0.5, 0.5),
                Vec3::new(0.5, 0.5, 0.5),
                Vec3::new(0.5, 0.5, -0.5),
            ],
        ),
        // -Y face
        (
            Vec3::NEG_Y,
            [
                Vec3::new(-0.5, -0.5, 0.5),
                Vec3::new(-0.5, -0.5, -0.5),
                Vec3::new(0.5, -0.5, -0.5),
                Vec3::new(0.5, -0.5, 0.5),
            ],
        ),
        // +Z face
        (
            Vec3::Z,
            [
                Vec3::new(-0.5, -0.5, 0.5),
                Vec3::new(0.5, -0.5, 0.5),
                Vec3::new(0.5, 0.5, 0.5),
                Vec3::new(-0.5, 0.5, 0.5),
            ],
        ),
        // -Z face
        (
            Vec3::NEG_Z,
            [
                Vec3::new(0.5, -0.5, -0.5),
                Vec3::new(-0.5, -0.5, -0.5),
                Vec3::new(-0.5, 0.5, -0.5),
                Vec3::new(0.5, 0.5, -0.5),
            ],
        ),
    ];

    for (normal, corners) in faces {
        let base = vertices.len() as u32;
        for pos in corners {
            vertices.push(CollisionVertex::new(pos, normal));
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    (vertices, indices)
}

/// Generate a unit sphere (centered at origin, radius 1)
fn generate_sphere(segments: u32, rings: u32) -> (Vec<CollisionVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for ring in 0..=rings {
        let phi = std::f32::consts::PI * ring as f32 / rings as f32;
        let y = phi.cos();
        let ring_radius = phi.sin();

        for seg in 0..=segments {
            let theta = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
            let x = ring_radius * theta.cos();
            let z = ring_radius * theta.sin();

            let pos = Vec3::new(x, y, z);
            vertices.push(CollisionVertex::new(pos, pos)); // For sphere, position = normal
        }
    }

    for ring in 0..rings {
        for seg in 0..segments {
            let current = ring * (segments + 1) + seg;
            let next = current + segments + 1;

            indices.push(current);
            indices.push(next);
            indices.push(current + 1);

            indices.push(current + 1);
            indices.push(next);
            indices.push(next + 1);
        }
    }

    (vertices, indices)
}

/// Generate a unit cylinder (centered at origin, radius 1, height 1 along Z)
fn generate_cylinder(segments: u32) -> (Vec<CollisionVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Top cap center
    let top_center_idx = vertices.len() as u32;
    vertices.push(CollisionVertex::new(Vec3::new(0.0, 0.0, 0.5), Vec3::Z));

    // Top cap ring
    let top_ring_start = vertices.len() as u32;
    for seg in 0..=segments {
        let theta = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
        let x = theta.cos();
        let y = theta.sin();
        vertices.push(CollisionVertex::new(Vec3::new(x, y, 0.5), Vec3::Z));
    }

    // Top cap triangles
    for seg in 0..segments {
        indices.push(top_center_idx);
        indices.push(top_ring_start + seg);
        indices.push(top_ring_start + seg + 1);
    }

    // Bottom cap center
    let bottom_center_idx = vertices.len() as u32;
    vertices.push(CollisionVertex::new(Vec3::new(0.0, 0.0, -0.5), Vec3::NEG_Z));

    // Bottom cap ring
    let bottom_ring_start = vertices.len() as u32;
    for seg in 0..=segments {
        let theta = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
        let x = theta.cos();
        let y = theta.sin();
        vertices.push(CollisionVertex::new(Vec3::new(x, y, -0.5), Vec3::NEG_Z));
    }

    // Bottom cap triangles (reverse winding)
    for seg in 0..segments {
        indices.push(bottom_center_idx);
        indices.push(bottom_ring_start + seg + 1);
        indices.push(bottom_ring_start + seg);
    }

    // Side vertices (need separate vertices for different normals)
    let side_top_start = vertices.len() as u32;
    for seg in 0..=segments {
        let theta = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
        let x = theta.cos();
        let y = theta.sin();
        let normal = Vec3::new(x, y, 0.0);
        vertices.push(CollisionVertex::new(Vec3::new(x, y, 0.5), normal));
    }

    let side_bottom_start = vertices.len() as u32;
    for seg in 0..=segments {
        let theta = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
        let x = theta.cos();
        let y = theta.sin();
        let normal = Vec3::new(x, y, 0.0);
        vertices.push(CollisionVertex::new(Vec3::new(x, y, -0.5), normal));
    }

    // Side triangles
    for seg in 0..segments {
        let t0 = side_top_start + seg;
        let t1 = side_top_start + seg + 1;
        let b0 = side_bottom_start + seg;
        let b1 = side_bottom_start + seg + 1;

        indices.push(t0);
        indices.push(b0);
        indices.push(t1);

        indices.push(t1);
        indices.push(b0);
        indices.push(b1);
    }

    (vertices, indices)
}

/// Generate a unit capsule (centered at origin, radius 1, total height 1 along Z)
fn generate_capsule(segments: u32, half_rings: u32) -> (Vec<CollisionVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // The capsule has cylindrical middle section and hemispherical caps
    // Total height = 1, so hemisphere radius + cylinder height + hemisphere radius = 1
    // With our scaling, we'll make the base shape have radius=1 and the caller scales

    let cylinder_half_height = 0.5; // Will be scaled

    // Top hemisphere
    for ring in 0..=half_rings {
        let phi = std::f32::consts::FRAC_PI_2 * ring as f32 / half_rings as f32;
        let y = phi.sin();
        let ring_radius = phi.cos();

        for seg in 0..=segments {
            let theta = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
            let x = ring_radius * theta.cos();
            let z_offset = ring_radius * theta.sin();

            let pos = Vec3::new(x, z_offset, cylinder_half_height + y);
            let normal = Vec3::new(x, z_offset, y).normalize();
            vertices.push(CollisionVertex::new(pos, normal));
        }
    }

    // Top hemisphere indices
    for ring in 0..half_rings {
        for seg in 0..segments {
            let current = ring * (segments + 1) + seg;
            let next = current + segments + 1;

            indices.push(current);
            indices.push(current + 1);
            indices.push(next);

            indices.push(current + 1);
            indices.push(next + 1);
            indices.push(next);
        }
    }

    // Cylinder side
    let cyl_top_start = vertices.len() as u32;
    for seg in 0..=segments {
        let theta = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
        let x = theta.cos();
        let y = theta.sin();
        let normal = Vec3::new(x, y, 0.0);
        vertices.push(CollisionVertex::new(
            Vec3::new(x, y, cylinder_half_height),
            normal,
        ));
    }

    let cyl_bottom_start = vertices.len() as u32;
    for seg in 0..=segments {
        let theta = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
        let x = theta.cos();
        let y = theta.sin();
        let normal = Vec3::new(x, y, 0.0);
        vertices.push(CollisionVertex::new(
            Vec3::new(x, y, -cylinder_half_height),
            normal,
        ));
    }

    // Cylinder side indices
    for seg in 0..segments {
        let t0 = cyl_top_start + seg;
        let t1 = cyl_top_start + seg + 1;
        let b0 = cyl_bottom_start + seg;
        let b1 = cyl_bottom_start + seg + 1;

        indices.push(t0);
        indices.push(b0);
        indices.push(t1);

        indices.push(t1);
        indices.push(b0);
        indices.push(b1);
    }

    // Bottom hemisphere
    let bottom_start = vertices.len() as u32;
    for ring in 0..=half_rings {
        let phi = std::f32::consts::FRAC_PI_2 * ring as f32 / half_rings as f32;
        let y = -phi.sin();
        let ring_radius = phi.cos();

        for seg in 0..=segments {
            let theta = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
            let x = ring_radius * theta.cos();
            let z_offset = ring_radius * theta.sin();

            let pos = Vec3::new(x, z_offset, -cylinder_half_height + y);
            let normal = Vec3::new(x, z_offset, y).normalize();
            vertices.push(CollisionVertex::new(pos, normal));
        }
    }

    // Bottom hemisphere indices
    for ring in 0..half_rings {
        for seg in 0..segments {
            let current = bottom_start + ring * (segments + 1) + seg;
            let next = current + segments + 1;

            indices.push(current);
            indices.push(next);
            indices.push(current + 1);

            indices.push(current + 1);
            indices.push(next);
            indices.push(next + 1);
        }
    }

    (vertices, indices)
}
