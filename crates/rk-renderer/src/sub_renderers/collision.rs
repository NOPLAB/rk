//! Collision shape visualization renderer

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::constants::{collision as constants, instances};
use crate::context::RenderContext;
use crate::pipeline::{PipelineConfig, create_camera_bind_group};
use crate::scene::Scene;
use crate::traits::SubRenderer;

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
    enabled: bool,
    initialized: bool,
    pipeline: Option<wgpu::RenderPipeline>,
    bind_group: Option<wgpu::BindGroup>,

    // Box geometry
    box_vertex_buffer: Option<wgpu::Buffer>,
    box_index_buffer: Option<wgpu::Buffer>,
    box_index_count: u32,
    box_instance_buffer: Option<wgpu::Buffer>,
    box_instances: Vec<CollisionInstance>,

    // Sphere geometry
    sphere_vertex_buffer: Option<wgpu::Buffer>,
    sphere_index_buffer: Option<wgpu::Buffer>,
    sphere_index_count: u32,
    sphere_instance_buffer: Option<wgpu::Buffer>,
    sphere_instances: Vec<CollisionInstance>,

    // Cylinder geometry
    cylinder_vertex_buffer: Option<wgpu::Buffer>,
    cylinder_index_buffer: Option<wgpu::Buffer>,
    cylinder_index_count: u32,
    cylinder_instance_buffer: Option<wgpu::Buffer>,
    cylinder_instances: Vec<CollisionInstance>,

    // Capsule geometry
    capsule_vertex_buffer: Option<wgpu::Buffer>,
    capsule_index_buffer: Option<wgpu::Buffer>,
    capsule_index_count: u32,
    capsule_instance_buffer: Option<wgpu::Buffer>,
    capsule_instances: Vec<CollisionInstance>,

    visible: bool,
}

impl CollisionRenderer {
    /// Creates a new collision renderer (uninitialized).
    pub fn new() -> Self {
        Self {
            enabled: true,
            initialized: false,
            pipeline: None,
            bind_group: None,
            box_vertex_buffer: None,
            box_index_buffer: None,
            box_index_count: 0,
            box_instance_buffer: None,
            box_instances: Vec::new(),
            sphere_vertex_buffer: None,
            sphere_index_buffer: None,
            sphere_index_count: 0,
            sphere_instance_buffer: None,
            sphere_instances: Vec::new(),
            cylinder_vertex_buffer: None,
            cylinder_index_buffer: None,
            cylinder_index_count: 0,
            cylinder_instance_buffer: None,
            cylinder_instances: Vec::new(),
            capsule_vertex_buffer: None,
            capsule_index_buffer: None,
            capsule_index_count: 0,
            capsule_instance_buffer: None,
            capsule_instances: Vec::new(),
            visible: true,
        }
    }

    /// Initialize the collision renderer with GPU resources.
    pub fn init(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
    ) {
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

        self.pipeline = Some(pipeline);
        self.bind_group = Some(bind_group);
        self.box_vertex_buffer = Some(box_vertex_buffer);
        self.box_index_buffer = Some(box_index_buffer);
        self.box_index_count = box_indices.len() as u32;
        self.box_instance_buffer = Some(box_instance_buffer);
        self.sphere_vertex_buffer = Some(sphere_vertex_buffer);
        self.sphere_index_buffer = Some(sphere_index_buffer);
        self.sphere_index_count = sphere_indices.len() as u32;
        self.sphere_instance_buffer = Some(sphere_instance_buffer);
        self.cylinder_vertex_buffer = Some(cylinder_vertex_buffer);
        self.cylinder_index_buffer = Some(cylinder_index_buffer);
        self.cylinder_index_count = cylinder_indices.len() as u32;
        self.cylinder_instance_buffer = Some(cylinder_instance_buffer);
        self.capsule_vertex_buffer = Some(capsule_vertex_buffer);
        self.capsule_index_buffer = Some(capsule_index_buffer);
        self.capsule_index_count = capsule_indices.len() as u32;
        self.capsule_instance_buffer = Some(capsule_instance_buffer);
        self.initialized = true;
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
        if !self.initialized {
            return;
        }
        if !self.box_instances.is_empty() {
            queue.write_buffer(
                self.box_instance_buffer.as_ref().unwrap(),
                0,
                bytemuck::cast_slice(&self.box_instances),
            );
        }
        if !self.sphere_instances.is_empty() {
            queue.write_buffer(
                self.sphere_instance_buffer.as_ref().unwrap(),
                0,
                bytemuck::cast_slice(&self.sphere_instances),
            );
        }
        if !self.cylinder_instances.is_empty() {
            queue.write_buffer(
                self.cylinder_instance_buffer.as_ref().unwrap(),
                0,
                bytemuck::cast_slice(&self.cylinder_instances),
            );
        }
        if !self.capsule_instances.is_empty() {
            queue.write_buffer(
                self.capsule_instance_buffer.as_ref().unwrap(),
                0,
                bytemuck::cast_slice(&self.capsule_instances),
            );
        }
    }

    /// Render all collision instances
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if !self.visible || !self.initialized {
            return;
        }

        let pipeline = self.pipeline.as_ref().unwrap();
        let bind_group = self.bind_group.as_ref().unwrap();

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);

        // Render boxes
        if !self.box_instances.is_empty() {
            let vb = self.box_vertex_buffer.as_ref().unwrap();
            let ib = self.box_index_buffer.as_ref().unwrap();
            let inst = self.box_instance_buffer.as_ref().unwrap();
            render_pass.set_vertex_buffer(0, vb.slice(..));
            render_pass.set_vertex_buffer(1, inst.slice(..));
            render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(
                0..self.box_index_count,
                0,
                0..self.box_instances.len() as u32,
            );
        }

        // Render spheres
        if !self.sphere_instances.is_empty() {
            let vb = self.sphere_vertex_buffer.as_ref().unwrap();
            let ib = self.sphere_index_buffer.as_ref().unwrap();
            let inst = self.sphere_instance_buffer.as_ref().unwrap();
            render_pass.set_vertex_buffer(0, vb.slice(..));
            render_pass.set_vertex_buffer(1, inst.slice(..));
            render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(
                0..self.sphere_index_count,
                0,
                0..self.sphere_instances.len() as u32,
            );
        }

        // Render cylinders
        if !self.cylinder_instances.is_empty() {
            let vb = self.cylinder_vertex_buffer.as_ref().unwrap();
            let ib = self.cylinder_index_buffer.as_ref().unwrap();
            let inst = self.cylinder_instance_buffer.as_ref().unwrap();
            render_pass.set_vertex_buffer(0, vb.slice(..));
            render_pass.set_vertex_buffer(1, inst.slice(..));
            render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(
                0..self.cylinder_index_count,
                0,
                0..self.cylinder_instances.len() as u32,
            );
        }

        // Render capsules
        if !self.capsule_instances.is_empty() {
            let vb = self.capsule_vertex_buffer.as_ref().unwrap();
            let ib = self.capsule_index_buffer.as_ref().unwrap();
            let inst = self.capsule_instance_buffer.as_ref().unwrap();
            render_pass.set_vertex_buffer(0, vb.slice(..));
            render_pass.set_vertex_buffer(1, inst.slice(..));
            render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(
                0..self.capsule_index_count,
                0,
                0..self.capsule_instances.len() as u32,
            );
        }
    }
}

impl Default for CollisionRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SubRenderer for CollisionRenderer {
    fn name(&self) -> &str {
        "collision"
    }

    fn priority(&self) -> i32 {
        super::priorities::COLLISION
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
        // Collision renderer doesn't need to respond to resize
    }

    fn prepare(&mut self, _ctx: &RenderContext, _scene: &Scene) {
        // Collision data is updated externally
    }

    fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, _scene: &Scene) {
        CollisionRenderer::render(self, pass);
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
