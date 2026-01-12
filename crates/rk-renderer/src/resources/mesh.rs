//! Mesh resource management.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::context::RenderContext;
use crate::scene::BoundingBox;
use crate::vertex::MeshVertex;

/// Handle to a mesh stored in the MeshManager.
///
/// Handles are lightweight and can be copied freely.
/// The actual mesh data is stored in the MeshManager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MeshHandle(u64);

impl MeshHandle {
    /// Returns the raw handle value.
    pub fn raw(&self) -> u64 {
        self.0
    }

    /// Creates a handle from a raw value (for deserialization).
    pub fn from_raw(value: u64) -> Self {
        Self(value)
    }
}

/// GPU mesh data.
pub struct GpuMesh {
    /// Vertex buffer containing position, normal, and color data.
    pub vertex_buffer: wgpu::Buffer,
    /// Index buffer (optional, for indexed drawing).
    pub index_buffer: Option<wgpu::Buffer>,
    /// Number of vertices.
    pub vertex_count: u32,
    /// Number of indices (0 if not indexed).
    pub index_count: u32,
    /// Bounding box of the mesh.
    pub bounds: BoundingBox,
}

impl GpuMesh {
    /// Returns true if this mesh uses indexed drawing.
    pub fn is_indexed(&self) -> bool {
        self.index_buffer.is_some() && self.index_count > 0
    }
}

/// CPU mesh data for uploading to GPU.
#[derive(Debug, Clone)]
pub struct MeshData {
    /// Vertex data.
    pub vertices: Vec<MeshVertex>,
    /// Index data (optional).
    pub indices: Option<Vec<u32>>,
    /// Bounding box.
    pub bounds: BoundingBox,
}

impl MeshData {
    /// Creates a new mesh data from vertices (non-indexed).
    pub fn new(vertices: Vec<MeshVertex>) -> Self {
        let bounds = Self::compute_bounds(&vertices);
        Self {
            vertices,
            indices: None,
            bounds,
        }
    }

    /// Creates a new indexed mesh data.
    pub fn indexed(vertices: Vec<MeshVertex>, indices: Vec<u32>) -> Self {
        let bounds = Self::compute_bounds(&vertices);
        Self {
            vertices,
            indices: Some(indices),
            bounds,
        }
    }

    /// Sets the bounding box explicitly.
    pub fn with_bounds(mut self, bounds: BoundingBox) -> Self {
        self.bounds = bounds;
        self
    }

    fn compute_bounds(vertices: &[MeshVertex]) -> BoundingBox {
        if vertices.is_empty() {
            return BoundingBox::empty();
        }

        let mut min = glam::Vec3::splat(f32::INFINITY);
        let mut max = glam::Vec3::splat(f32::NEG_INFINITY);

        for v in vertices {
            let pos = glam::Vec3::from(v.position);
            min = min.min(pos);
            max = max.max(pos);
        }

        BoundingBox::new(min, max)
    }
}

/// Manager for GPU mesh resources.
///
/// The MeshManager provides handle-based access to GPU mesh data,
/// enabling efficient resource sharing and deferred loading.
pub struct MeshManager {
    meshes: HashMap<MeshHandle, GpuMesh>,
    next_handle: AtomicU64,
}

impl MeshManager {
    /// Creates a new mesh manager.
    pub fn new() -> Self {
        Self {
            meshes: HashMap::new(),
            next_handle: AtomicU64::new(1),
        }
    }

    /// Uploads mesh data to the GPU and returns a handle.
    pub fn create(&mut self, ctx: &RenderContext, data: &MeshData) -> MeshHandle {
        let handle = MeshHandle(self.next_handle.fetch_add(1, Ordering::Relaxed));

        let vertex_buffer = ctx.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Vertex Buffer"),
            contents: bytemuck::cast_slice(&data.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = data.indices.as_ref().map(|indices| {
            ctx.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh Index Buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            })
        });

        let gpu_mesh = GpuMesh {
            vertex_buffer,
            index_buffer,
            vertex_count: data.vertices.len() as u32,
            index_count: data.indices.as_ref().map(|i| i.len() as u32).unwrap_or(0),
            bounds: data.bounds,
        };

        self.meshes.insert(handle, gpu_mesh);
        handle
    }

    /// Gets a mesh by handle.
    pub fn get(&self, handle: MeshHandle) -> Option<&GpuMesh> {
        self.meshes.get(&handle)
    }

    /// Removes a mesh from the manager.
    ///
    /// The GPU resources will be released when the GpuMesh is dropped.
    pub fn remove(&mut self, handle: MeshHandle) -> Option<GpuMesh> {
        self.meshes.remove(&handle)
    }

    /// Returns true if the manager contains a mesh with the given handle.
    pub fn contains(&self, handle: MeshHandle) -> bool {
        self.meshes.contains_key(&handle)
    }

    /// Returns the number of meshes in the manager.
    pub fn len(&self) -> usize {
        self.meshes.len()
    }

    /// Returns true if the manager is empty.
    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty()
    }

    /// Clears all meshes from the manager.
    pub fn clear(&mut self) {
        self.meshes.clear();
    }
}

impl Default for MeshManager {
    fn default() -> Self {
        Self::new()
    }
}
