//! CAD Kernel trait definitions
//!
//! These traits define the interface that all CAD kernels must implement.

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Error type for CAD kernel operations
#[derive(Debug, Clone, Error)]
pub enum CadError {
    #[error("Invalid profile: {0}")]
    InvalidProfile(String),

    #[error("Boolean operation failed: {0}")]
    BooleanFailed(String),

    #[error("Tessellation failed: {0}")]
    TessellationFailed(String),

    #[error("Kernel not available: {0}")]
    KernelNotAvailable(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

/// Result type for CAD operations
pub type CadResult<T> = Result<T, CadError>;

/// A tessellated mesh output from the CAD kernel
#[derive(Debug, Clone, Default)]
pub struct TessellatedMesh {
    /// Vertex positions (3 floats per vertex)
    pub vertices: Vec<[f32; 3]>,
    /// Vertex normals (3 floats per vertex)
    pub normals: Vec<[f32; 3]>,
    /// Triangle indices (3 indices per triangle)
    pub indices: Vec<u32>,
}

impl TessellatedMesh {
    /// Create an empty tessellated mesh
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the mesh is empty
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Get the number of triangles
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// A 2D wire (closed loop of edges) for extrusion profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wire2D {
    /// Unique identifier
    pub id: Uuid,
    /// Points defining the wire (in order)
    pub points: Vec<Vec2>,
    /// Whether the wire is closed
    pub closed: bool,
}

impl Wire2D {
    /// Create a new wire from points
    pub fn new(points: Vec<Vec2>, closed: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            points,
            closed,
        }
    }

    /// Create a rectangle wire
    pub fn rectangle(center: Vec2, width: f32, height: f32) -> Self {
        let hw = width / 2.0;
        let hh = height / 2.0;
        Self::new(
            vec![
                center + Vec2::new(-hw, -hh),
                center + Vec2::new(hw, -hh),
                center + Vec2::new(hw, hh),
                center + Vec2::new(-hw, hh),
            ],
            true,
        )
    }

    /// Create a circle wire (approximated with segments)
    pub fn circle(center: Vec2, radius: f32, segments: u32) -> Self {
        let points: Vec<Vec2> = (0..segments)
            .map(|i| {
                let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
                center + Vec2::new(angle.cos() * radius, angle.sin() * radius)
            })
            .collect();
        Self::new(points, true)
    }
}

/// A 3D solid body
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Solid {
    /// Unique identifier
    pub id: Uuid,
    /// Internal marker for kernel data (actual data stored in kernel)
    #[serde(skip)]
    has_kernel_data: bool,
}

impl Clone for Solid {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            has_kernel_data: self.has_kernel_data,
        }
    }
}

impl Solid {
    /// Create a new solid with the given ID
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            has_kernel_data: false,
        }
    }

    /// Mark that this solid has kernel data
    pub fn with_kernel_data(mut self) -> Self {
        self.has_kernel_data = true;
        self
    }

    /// Check if this solid has kernel data
    pub fn has_kernel_data(&self) -> bool {
        self.has_kernel_data
    }
}

/// Axis definition for revolve operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Axis3D {
    /// Origin point of the axis
    pub origin: Vec3,
    /// Direction of the axis (normalized)
    pub direction: Vec3,
}

impl Axis3D {
    /// Create an axis from origin and direction
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    /// X axis at origin
    pub fn x() -> Self {
        Self::new(Vec3::ZERO, Vec3::X)
    }

    /// Y axis at origin
    pub fn y() -> Self {
        Self::new(Vec3::ZERO, Vec3::Y)
    }

    /// Z axis at origin
    pub fn z() -> Self {
        Self::new(Vec3::ZERO, Vec3::Z)
    }
}

/// Boolean operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BooleanType {
    /// Union (add)
    Union,
    /// Subtraction (cut)
    Subtract,
    /// Intersection (common)
    Intersect,
}

/// The main CAD kernel trait
///
/// Implementations of this trait provide the actual geometry operations
/// using different backends (OpenCASCADE, Truck, etc.)
pub trait CadKernel: Send + Sync {
    /// Get the name of this kernel
    fn name(&self) -> &str;

    /// Check if the kernel is available
    fn is_available(&self) -> bool;

    /// Extrude a 2D profile along a direction
    ///
    /// # Arguments
    /// * `profile` - The 2D wire profile to extrude
    /// * `plane_origin` - The origin of the sketch plane in 3D
    /// * `plane_normal` - The normal of the sketch plane
    /// * `direction` - The extrusion direction (local to plane)
    /// * `distance` - The extrusion distance
    fn extrude(
        &self,
        profile: &Wire2D,
        plane_origin: Vec3,
        plane_normal: Vec3,
        direction: Vec3,
        distance: f32,
    ) -> CadResult<Solid>;

    /// Revolve a 2D profile around an axis
    ///
    /// # Arguments
    /// * `profile` - The 2D wire profile to revolve
    /// * `plane_origin` - The origin of the sketch plane in 3D
    /// * `plane_normal` - The normal of the sketch plane
    /// * `axis` - The rotation axis
    /// * `angle` - The rotation angle in radians
    fn revolve(
        &self,
        profile: &Wire2D,
        plane_origin: Vec3,
        plane_normal: Vec3,
        axis: &Axis3D,
        angle: f32,
    ) -> CadResult<Solid>;

    /// Perform a boolean operation on two solids
    ///
    /// # Arguments
    /// * `a` - The first solid
    /// * `b` - The second solid
    /// * `op` - The boolean operation type
    fn boolean(&self, a: &Solid, b: &Solid, op: BooleanType) -> CadResult<Solid>;

    /// Tessellate a solid into triangles
    ///
    /// # Arguments
    /// * `solid` - The solid to tessellate
    /// * `tolerance` - The tessellation tolerance (lower = more triangles)
    fn tessellate(&self, solid: &Solid, tolerance: f32) -> CadResult<TessellatedMesh>;

    /// Create a box primitive
    fn create_box(&self, center: Vec3, size: Vec3) -> CadResult<Solid>;

    /// Create a cylinder primitive
    fn create_cylinder(
        &self,
        center: Vec3,
        radius: f32,
        height: f32,
        axis: Vec3,
    ) -> CadResult<Solid>;

    /// Create a sphere primitive
    fn create_sphere(&self, center: Vec3, radius: f32) -> CadResult<Solid>;
}

/// A null kernel that always returns errors (used when no kernel is available)
#[derive(Debug, Default)]
pub struct NullKernel;

impl CadKernel for NullKernel {
    fn name(&self) -> &str {
        "null"
    }

    fn is_available(&self) -> bool {
        false
    }

    fn extrude(
        &self,
        _profile: &Wire2D,
        _plane_origin: Vec3,
        _plane_normal: Vec3,
        _direction: Vec3,
        _distance: f32,
    ) -> CadResult<Solid> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available".into(),
        ))
    }

    fn revolve(
        &self,
        _profile: &Wire2D,
        _plane_origin: Vec3,
        _plane_normal: Vec3,
        _axis: &Axis3D,
        _angle: f32,
    ) -> CadResult<Solid> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available".into(),
        ))
    }

    fn boolean(&self, _a: &Solid, _b: &Solid, _op: BooleanType) -> CadResult<Solid> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available".into(),
        ))
    }

    fn tessellate(&self, _solid: &Solid, _tolerance: f32) -> CadResult<TessellatedMesh> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available".into(),
        ))
    }

    fn create_box(&self, _center: Vec3, _size: Vec3) -> CadResult<Solid> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available".into(),
        ))
    }

    fn create_cylinder(
        &self,
        _center: Vec3,
        _radius: f32,
        _height: f32,
        _axis: Vec3,
    ) -> CadResult<Solid> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available".into(),
        ))
    }

    fn create_sphere(&self, _center: Vec3, _radius: f32) -> CadResult<Solid> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available".into(),
        ))
    }
}

/// Get the default CAD kernel based on available features
pub fn default_kernel() -> Box<dyn CadKernel> {
    #[cfg(feature = "opencascade")]
    {
        return Box::new(super::OpenCascadeKernel::new());
    }

    #[cfg(feature = "truck")]
    {
        return Box::new(super::TruckKernel::new());
    }

    #[cfg(not(any(feature = "opencascade", feature = "truck")))]
    {
        Box::new(NullKernel)
    }
}
