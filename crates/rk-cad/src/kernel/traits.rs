//! CAD Kernel trait definitions
//!
//! These traits define the interface that all CAD kernels must implement.

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Unique identifier for an edge within a solid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId {
    /// ID of the solid this edge belongs to
    pub solid_id: Uuid,
    /// Index of the edge within the solid
    pub index: u32,
}

impl EdgeId {
    /// Create a new edge ID
    pub fn new(solid_id: Uuid, index: u32) -> Self {
        Self { solid_id, index }
    }
}

/// Unique identifier for a face within a solid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FaceId {
    /// ID of the solid this face belongs to
    pub solid_id: Uuid,
    /// Index of the face within the solid
    pub index: u32,
}

impl FaceId {
    /// Create a new face ID
    pub fn new(solid_id: Uuid, index: u32) -> Self {
        Self { solid_id, index }
    }
}

/// Information about an edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeInfo {
    /// Unique identifier for this edge
    pub id: EdgeId,
    /// Start point of the edge
    pub start: Vec3,
    /// End point of the edge
    pub end: Vec3,
    /// Midpoint of the edge
    pub midpoint: Vec3,
    /// Length of the edge
    pub length: f32,
}

impl EdgeInfo {
    /// Create a new edge info
    pub fn new(id: EdgeId, start: Vec3, end: Vec3) -> Self {
        let midpoint = (start + end) * 0.5;
        let length = (end - start).length();
        Self {
            id,
            start,
            end,
            midpoint,
            length,
        }
    }
}

/// Information about a face
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceInfo {
    /// Unique identifier for this face
    pub id: FaceId,
    /// Center point of the face
    pub center: Vec3,
    /// Normal vector of the face
    pub normal: Vec3,
    /// Approximate area of the face
    pub area: f32,
}

impl FaceInfo {
    /// Create a new face info
    pub fn new(id: FaceId, center: Vec3, normal: Vec3, area: f32) -> Self {
        Self {
            id,
            center,
            normal: normal.normalize(),
            area,
        }
    }
}

/// Corner type for sweep operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SweepCornerType {
    /// Natural corners (default)
    #[default]
    Natural,
    /// Rounded corners
    Round,
    /// Angular corners
    Angular,
}

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

    #[error("File I/O error: {0}")]
    FileIo(String),

    #[error("STEP import failed: {0}")]
    StepImport(String),

    #[error("STEP export failed: {0}")]
    StepExport(String),
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

// ========== STEP File I/O Types ==========

/// Options for STEP file import
#[derive(Debug, Clone, Default)]
pub struct StepImportOptions {
    /// Tessellation tolerance for mesh conversion (lower = finer mesh)
    /// Default: 0.1
    pub tessellation_tolerance: Option<f32>,
    /// Whether to import as solids (true) or directly tessellate to meshes (false)
    /// Default: false (tessellate immediately)
    pub import_as_solids: bool,
}

/// Options for STEP file export
#[derive(Debug, Clone, Default)]
pub struct StepExportOptions {
    /// Application name in STEP header
    pub author: Option<String>,
    /// Organization name in STEP header
    pub organization: Option<String>,
}

/// Result of STEP import containing multiple bodies
#[derive(Debug, Clone, Default)]
pub struct StepImportResult {
    /// Imported solids (if import_as_solids was true)
    pub solids: Vec<Solid>,
    /// Pre-tessellated meshes (if import_as_solids was false)
    pub meshes: Vec<TessellatedMesh>,
    /// Names extracted from STEP entities (if available)
    pub names: Vec<Option<String>>,
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
    /// * `plane_x_axis` - The X axis of the sketch plane (for 2D to 3D mapping)
    /// * `direction` - The extrusion direction (local to plane)
    /// * `distance` - The extrusion distance
    fn extrude(
        &self,
        profile: &Wire2D,
        plane_origin: Vec3,
        plane_normal: Vec3,
        plane_x_axis: Vec3,
        direction: Vec3,
        distance: f32,
    ) -> CadResult<Solid>;

    /// Revolve a 2D profile around an axis
    ///
    /// # Arguments
    /// * `profile` - The 2D wire profile to revolve
    /// * `plane_origin` - The origin of the sketch plane in 3D
    /// * `plane_normal` - The normal of the sketch plane
    /// * `plane_x_axis` - The X axis of the sketch plane (for 2D to 3D mapping)
    /// * `axis` - The rotation axis
    /// * `angle` - The rotation angle in radians
    fn revolve(
        &self,
        profile: &Wire2D,
        plane_origin: Vec3,
        plane_normal: Vec3,
        plane_x_axis: Vec3,
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

    // ========== Edge/Face Query Methods ==========

    /// Get all edges of a solid with their geometric information
    ///
    /// # Arguments
    /// * `solid` - The solid to query
    fn get_edges(&self, solid: &Solid) -> CadResult<Vec<EdgeInfo>>;

    /// Get all faces of a solid with their geometric information
    ///
    /// # Arguments
    /// * `solid` - The solid to query
    fn get_faces(&self, solid: &Solid) -> CadResult<Vec<FaceInfo>>;

    // ========== Fillet/Chamfer Methods ==========

    /// Apply fillet (rounded edge) to selected edges
    ///
    /// # Arguments
    /// * `solid` - The solid to modify
    /// * `edges` - Edge IDs to fillet
    /// * `radius` - Fillet radius
    fn fillet(&self, solid: &Solid, edges: &[EdgeId], radius: f32) -> CadResult<Solid>;

    /// Apply chamfer (beveled edge) to selected edges
    ///
    /// # Arguments
    /// * `solid` - The solid to modify
    /// * `edges` - Edge IDs to chamfer
    /// * `distance` - Chamfer distance
    fn chamfer(&self, solid: &Solid, edges: &[EdgeId], distance: f32) -> CadResult<Solid>;

    // ========== Shell Method ==========

    /// Create a hollow shell from a solid
    ///
    /// # Arguments
    /// * `solid` - The solid to shell
    /// * `thickness` - Wall thickness (positive = inward, negative = outward)
    /// * `faces_to_remove` - Faces to remove (create openings)
    fn shell(&self, solid: &Solid, thickness: f32, faces_to_remove: &[FaceId]) -> CadResult<Solid>;

    // ========== Sweep/Loft Methods ==========

    /// Sweep a profile along a path
    ///
    /// # Arguments
    /// * `profile` - The 2D profile to sweep
    /// * `profile_plane_origin` - Origin of the profile plane
    /// * `profile_plane_normal` - Normal of the profile plane
    /// * `path` - The 3D path to sweep along (as Wire2D on XY plane)
    /// * `path_plane_origin` - Origin of the path plane
    /// * `path_plane_normal` - Normal of the path plane
    fn sweep(
        &self,
        profile: &Wire2D,
        profile_plane_origin: Vec3,
        profile_plane_normal: Vec3,
        path: &Wire2D,
        path_plane_origin: Vec3,
        path_plane_normal: Vec3,
    ) -> CadResult<Solid>;

    /// Loft between multiple profiles
    ///
    /// # Arguments
    /// * `profiles` - List of profiles with their plane information
    /// * `create_solid` - Whether to create a solid (true) or shell (false)
    /// * `ruled` - Whether to use ruled surfaces
    fn loft(
        &self,
        profiles: &[(Wire2D, Vec3, Vec3)], // (profile, plane_origin, plane_normal)
        create_solid: bool,
        ruled: bool,
    ) -> CadResult<Solid>;

    // ========== STEP File I/O Methods ==========

    /// Import a STEP file
    ///
    /// # Arguments
    /// * `path` - Path to the STEP file
    /// * `options` - Import options
    fn import_step(
        &self,
        path: &std::path::Path,
        options: &StepImportOptions,
    ) -> CadResult<StepImportResult>;

    /// Export a solid to a STEP file
    ///
    /// # Arguments
    /// * `solid` - The solid to export
    /// * `path` - Output file path
    /// * `options` - Export options
    fn export_step(
        &self,
        solid: &Solid,
        path: &std::path::Path,
        options: &StepExportOptions,
    ) -> CadResult<()>;

    /// Export multiple solids to a single STEP file
    ///
    /// # Arguments
    /// * `solids` - The solids to export
    /// * `path` - Output file path
    /// * `options` - Export options
    fn export_step_multi(
        &self,
        solids: &[&Solid],
        path: &std::path::Path,
        options: &StepExportOptions,
    ) -> CadResult<()>;
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
        _plane_x_axis: Vec3,
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
        _plane_x_axis: Vec3,
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

    fn get_edges(&self, _solid: &Solid) -> CadResult<Vec<EdgeInfo>> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available".into(),
        ))
    }

    fn get_faces(&self, _solid: &Solid) -> CadResult<Vec<FaceInfo>> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available".into(),
        ))
    }

    fn fillet(&self, _solid: &Solid, _edges: &[EdgeId], _radius: f32) -> CadResult<Solid> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available".into(),
        ))
    }

    fn chamfer(&self, _solid: &Solid, _edges: &[EdgeId], _distance: f32) -> CadResult<Solid> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available".into(),
        ))
    }

    fn shell(
        &self,
        _solid: &Solid,
        _thickness: f32,
        _faces_to_remove: &[FaceId],
    ) -> CadResult<Solid> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available".into(),
        ))
    }

    fn sweep(
        &self,
        _profile: &Wire2D,
        _profile_plane_origin: Vec3,
        _profile_plane_normal: Vec3,
        _path: &Wire2D,
        _path_plane_origin: Vec3,
        _path_plane_normal: Vec3,
    ) -> CadResult<Solid> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available".into(),
        ))
    }

    fn loft(
        &self,
        _profiles: &[(Wire2D, Vec3, Vec3)],
        _create_solid: bool,
        _ruled: bool,
    ) -> CadResult<Solid> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available".into(),
        ))
    }

    fn import_step(
        &self,
        _path: &std::path::Path,
        _options: &StepImportOptions,
    ) -> CadResult<StepImportResult> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available for STEP import".into(),
        ))
    }

    fn export_step(
        &self,
        _solid: &Solid,
        _path: &std::path::Path,
        _options: &StepExportOptions,
    ) -> CadResult<()> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available for STEP export".into(),
        ))
    }

    fn export_step_multi(
        &self,
        _solids: &[&Solid],
        _path: &std::path::Path,
        _options: &StepExportOptions,
    ) -> CadResult<()> {
        Err(CadError::KernelNotAvailable(
            "No CAD kernel available for STEP export".into(),
        ))
    }
}

/// Get the default CAD kernel based on available features
pub fn default_kernel() -> Box<dyn CadKernel> {
    #[cfg(feature = "opencascade")]
    {
        Box::new(super::OpenCascadeKernel::new())
    }

    #[cfg(all(feature = "truck", not(feature = "opencascade")))]
    {
        Box::new(super::TruckKernel::new())
    }

    #[cfg(not(any(feature = "opencascade", feature = "truck")))]
    {
        Box::new(NullKernel)
    }
}
