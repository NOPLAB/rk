//! Vertex attribute utilities
//!
//! This module provides utilities for defining vertex attributes with
//! type-safe offset calculation using `std::mem::offset_of!`.

/// Creates a vertex attribute with the offset calculated from the struct field.
///
/// This macro uses `std::mem::offset_of!` to ensure that the offset is always
/// correct, even if the struct layout changes.
///
/// # Example
///
/// ```ignore
/// #[repr(C)]
/// struct MyVertex {
///     position: [f32; 3],
///     normal: [f32; 3],
///     color: [f32; 4],
/// }
///
/// const VERTEX_ATTRIBUTES: &[wgpu::VertexAttribute] = &[
///     vertex_attr!(MyVertex, position, 0, Float32x3),
///     vertex_attr!(MyVertex, normal, 1, Float32x3),
///     vertex_attr!(MyVertex, color, 2, Float32x4),
/// ];
/// ```
#[macro_export]
macro_rules! vertex_attr {
    ($struct:ty, $field:ident, $location:expr, $format:ident) => {
        wgpu::VertexAttribute {
            offset: std::mem::offset_of!($struct, $field) as u64,
            shader_location: $location,
            format: wgpu::VertexFormat::$format,
        }
    };
}

/// Creates a vertex buffer layout from attributes.
///
/// # Type Parameters
///
/// * `T` - The vertex struct type, used to calculate the array stride.
///
/// # Arguments
///
/// * `attributes` - Slice of vertex attributes.
/// * `step_mode` - Whether this buffer is per-vertex or per-instance.
pub fn vertex_buffer_layout<T>(
    attributes: &[wgpu::VertexAttribute],
    step_mode: wgpu::VertexStepMode,
) -> wgpu::VertexBufferLayout<'_> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<T>() as u64,
        step_mode,
        attributes,
    }
}

/// Common vertex format for position + color vertices (used by grid, axis).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PositionColorVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl PositionColorVertex {
    pub const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &[
        wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x3,
        },
        wgpu::VertexAttribute {
            offset: std::mem::size_of::<[f32; 3]>() as u64,
            shader_location: 1,
            format: wgpu::VertexFormat::Float32x3,
        },
    ];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

/// Common vertex format for position-only vertices (used by marker spheres).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PositionVertex {
    pub position: [f32; 3],
}

impl PositionVertex {
    pub const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &[wgpu::VertexAttribute {
        offset: 0,
        shader_location: 0,
        format: wgpu::VertexFormat::Float32x3,
    }];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

/// Helper to create instance buffer layout for Mat4 transform.
///
/// Many renderers use a transform matrix as instance data. This helper
/// creates the vertex attributes for a Mat4 stored as 4 consecutive Float32x4.
pub fn mat4_instance_attributes(start_location: u32) -> [wgpu::VertexAttribute; 4] {
    [
        wgpu::VertexAttribute {
            offset: 0,
            shader_location: start_location,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: 16,
            shader_location: start_location + 1,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: 32,
            shader_location: start_location + 2,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: 48,
            shader_location: start_location + 3,
            format: wgpu::VertexFormat::Float32x4,
        },
    ]
}

/// Vertex for mesh rendering with position, normal, and color.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}

impl MeshVertex {
    pub const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &[
        wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x3,
        },
        wgpu::VertexAttribute {
            offset: std::mem::size_of::<[f32; 3]>() as u64,
            shader_location: 1,
            format: wgpu::VertexFormat::Float32x3,
        },
        wgpu::VertexAttribute {
            offset: (std::mem::size_of::<[f32; 3]>() * 2) as u64,
            shader_location: 2,
            format: wgpu::VertexFormat::Float32x4,
        },
    ];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}
