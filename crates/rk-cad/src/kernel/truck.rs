//! Truck CAD Kernel Backend
//!
//! Pure Rust B-Rep kernel using the Truck library.
//!
//! Supports basic solid modeling capabilities including extrude, revolve,
//! and boolean operations (union, intersection).

use glam::{Vec2, Vec3};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

use truck_meshalgo::prelude::*;
use truck_modeling::{Point3, Solid as TruckSolid, Vector3, Vertex, Wire, builder};
use truck_shapeops::{and as solid_and, or as solid_or};

use super::{
    Axis3D, BooleanType, CadError, CadKernel, CadResult, EdgeId, EdgeInfo, FaceId, FaceInfo, Solid,
    StepExportOptions, StepImportOptions, StepImportResult, TessellatedMesh, Wire2D,
};

/// Truck-based CAD kernel
pub struct TruckKernel {
    /// Storage for solid data (keyed by UUID)
    solids: Mutex<HashMap<Uuid, TruckSolid>>,
}

impl TruckKernel {
    /// Create a new Truck kernel
    pub fn new() -> Self {
        Self {
            solids: Mutex::new(HashMap::new()),
        }
    }

    /// Store a solid and return a Solid reference
    fn store_solid(&self, solid: TruckSolid) -> Solid {
        let id = Uuid::new_v4();
        let mut solids = self.solids.lock().unwrap();
        solids.insert(id, solid);
        Solid::new(id).with_kernel_data()
    }

    /// Get a stored solid by ID
    #[allow(dead_code)]
    fn get_solid(&self, id: Uuid) -> Option<TruckSolid> {
        let solids = self.solids.lock().unwrap();
        solids.get(&id).cloned()
    }

    /// Convert 2D points to 3D points on a plane
    fn points_to_3d(
        &self,
        points: &[Vec2],
        plane_origin: Vec3,
        plane_x_axis: Vec3,
        plane_y_axis: Vec3,
    ) -> Vec<Point3> {
        let origin = Point3::new(
            plane_origin.x as f64,
            plane_origin.y as f64,
            plane_origin.z as f64,
        );

        // Use the provided x_axis and y_axis for the plane
        let u = Vector3::new(
            plane_x_axis.x as f64,
            plane_x_axis.y as f64,
            plane_x_axis.z as f64,
        );
        let v = Vector3::new(
            plane_y_axis.x as f64,
            plane_y_axis.y as f64,
            plane_y_axis.z as f64,
        );

        // Transform 2D points to 3D
        points
            .iter()
            .map(|p| {
                let x = p.x as f64;
                let y = p.y as f64;
                origin + u * x + v * y
            })
            .collect()
    }

    /// Create a wire from 2D points
    fn create_wire(
        &self,
        profile: &Wire2D,
        plane_origin: Vec3,
        plane_x_axis: Vec3,
        plane_y_axis: Vec3,
    ) -> Wire {
        let points_3d =
            self.points_to_3d(&profile.points, plane_origin, plane_x_axis, plane_y_axis);

        // Create vertices
        let vertices: Vec<Vertex> = points_3d.iter().map(|p| builder::vertex(*p)).collect();

        // Create edges between consecutive vertices
        let n = vertices.len();
        let edges: Vec<_> = (0..n)
            .map(|i| {
                let v0 = &vertices[i];
                let v1 = &vertices[(i + 1) % n];
                builder::line(v0, v1)
            })
            .collect();

        edges.into()
    }
}

impl Default for TruckKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl CadKernel for TruckKernel {
    fn name(&self) -> &str {
        "truck"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn extrude(
        &self,
        profile: &Wire2D,
        plane_origin: Vec3,
        plane_x_axis: Vec3,
        plane_y_axis: Vec3,
        direction: Vec3,
        distance: f32,
    ) -> CadResult<Solid> {
        if profile.points.len() < 3 {
            return Err(CadError::InvalidProfile(
                "Profile must have at least 3 points".into(),
            ));
        }

        // Create wire from profile
        let wire = self.create_wire(profile, plane_origin, plane_x_axis, plane_y_axis);

        // Create extrusion direction vector
        let dir = Vector3::new(
            direction.x as f64 * distance as f64,
            direction.y as f64 * distance as f64,
            direction.z as f64 * distance as f64,
        );

        // Create a face from the wire
        let face = builder::try_attach_plane(&[wire])
            .map_err(|e| CadError::OperationFailed(format!("Failed to create face: {:?}", e)))?;

        // Extrude the face
        let solid = builder::tsweep(&face, dir);

        Ok(self.store_solid(solid))
    }

    fn revolve(
        &self,
        profile: &Wire2D,
        plane_origin: Vec3,
        plane_x_axis: Vec3,
        plane_y_axis: Vec3,
        axis: &Axis3D,
        angle: f32,
    ) -> CadResult<Solid> {
        if profile.points.len() < 3 {
            return Err(CadError::InvalidProfile(
                "Profile must have at least 3 points".into(),
            ));
        }

        // Create wire from profile
        let wire = self.create_wire(profile, plane_origin, plane_x_axis, plane_y_axis);

        // Create axis
        let axis_origin = Point3::new(
            axis.origin.x as f64,
            axis.origin.y as f64,
            axis.origin.z as f64,
        );
        let axis_dir = Vector3::new(
            axis.direction.x as f64,
            axis.direction.y as f64,
            axis.direction.z as f64,
        );

        // Create a face from the wire
        let face = builder::try_attach_plane(&[wire])
            .map_err(|e| CadError::OperationFailed(format!("Failed to create face: {:?}", e)))?;

        // Revolve the face
        let solid = builder::rsweep(
            &face,
            axis_origin,
            axis_dir,
            truck_modeling::Rad(angle as f64),
        );

        Ok(self.store_solid(solid))
    }

    fn boolean(&self, a: &Solid, b: &Solid, op: BooleanType) -> CadResult<Solid> {
        // Get the stored solids
        let solids = self.solids.lock().unwrap();
        let solid_a = solids
            .get(&a.id)
            .ok_or_else(|| CadError::OperationFailed("First solid not found".into()))?
            .clone();
        let solid_b = solids
            .get(&b.id)
            .ok_or_else(|| CadError::OperationFailed("Second solid not found".into()))?
            .clone();
        drop(solids); // Release lock before operation

        // Tolerance for boolean operations
        let tolerance = 1e-6;

        let result = match op {
            BooleanType::Union => solid_or(&solid_a, &solid_b, tolerance)
                .ok_or_else(|| CadError::OperationFailed("Union operation failed".into()))?,
            BooleanType::Intersect => solid_and(&solid_a, &solid_b, tolerance)
                .ok_or_else(|| CadError::OperationFailed("Intersection operation failed".into()))?,
            BooleanType::Subtract => {
                // Subtract (A - B) is not directly supported by truck-shapeops
                // It would require A ∩ complement(B), but complement is not available
                return Err(CadError::OperationFailed(
                    "Subtract (Cut) operation is not supported by Truck kernel. \
                    Only Union and Intersection are available."
                        .into(),
                ));
            }
        };

        Ok(self.store_solid(result))
    }

    fn tessellate(&self, solid: &Solid, tolerance: f32) -> CadResult<TessellatedMesh> {
        // Get the stored solid
        let solids = self.solids.lock().unwrap();
        let truck_solid = solids
            .get(&solid.id)
            .ok_or_else(|| CadError::TessellationFailed("Solid not found".into()))?;

        // Tessellate using truck-meshalgo and convert to polygon mesh
        let meshed_solid = truck_solid.triangulation(tolerance as f64);
        let mut mesh = meshed_solid.to_polygon();

        // Add smooth normals for better rendering
        mesh.add_smooth_normals(0.5, true);

        // Clean up the mesh
        mesh.put_together_same_attrs(1e-6);
        mesh.remove_degenerate_faces();
        mesh.remove_unused_attrs();

        // Extract vertices
        let vertices: Vec<[f32; 3]> = mesh
            .positions()
            .iter()
            .map(|p| [p.x as f32, p.y as f32, p.z as f32])
            .collect();

        // Extract normals
        let normals: Vec<[f32; 3]> = mesh
            .normals()
            .iter()
            .map(|n| [n.x as f32, n.y as f32, n.z as f32])
            .collect();

        // Extract indices from triangle faces
        let mut indices: Vec<u32> = Vec::new();
        for tri in mesh.tri_faces() {
            indices.push(tri[0].pos as u32);
            indices.push(tri[1].pos as u32);
            indices.push(tri[2].pos as u32);
        }
        // Convert quad faces to triangles
        for quad in mesh.quad_faces() {
            indices.push(quad[0].pos as u32);
            indices.push(quad[1].pos as u32);
            indices.push(quad[2].pos as u32);
            indices.push(quad[0].pos as u32);
            indices.push(quad[2].pos as u32);
            indices.push(quad[3].pos as u32);
        }

        Ok(TessellatedMesh {
            vertices,
            normals,
            indices,
        })
    }

    fn create_box(&self, center: Vec3, size: Vec3) -> CadResult<Solid> {
        let half = size * 0.5;
        let min = center - half;

        let vertex = builder::vertex(Point3::new(min.x as f64, min.y as f64, min.z as f64));
        let edge = builder::tsweep(&vertex, Vector3::new(size.x as f64, 0.0, 0.0));
        let face = builder::tsweep(&edge, Vector3::new(0.0, size.y as f64, 0.0));
        let solid = builder::tsweep(&face, Vector3::new(0.0, 0.0, size.z as f64));

        Ok(self.store_solid(solid))
    }

    fn create_cylinder(
        &self,
        center: Vec3,
        radius: f32,
        height: f32,
        axis: Vec3,
    ) -> CadResult<Solid> {
        let axis_normalized = axis.normalize();
        let half_height = height / 2.0;
        let base_center = center - axis_normalized * half_height;

        // Create circle at base
        let wire = Wire2D::circle(Vec2::ZERO, radius, 32);

        // Calculate a perpendicular x_axis and y_axis for the plane
        let x_axis = if axis_normalized.z.abs() < 0.9 {
            axis_normalized.cross(Vec3::Z).normalize()
        } else {
            axis_normalized.cross(Vec3::X).normalize()
        };
        let y_axis = axis_normalized.cross(x_axis).normalize();

        self.extrude(&wire, base_center, x_axis, y_axis, axis_normalized, height)
    }

    fn create_sphere(&self, center: Vec3, radius: f32) -> CadResult<Solid> {
        // Create a semi-circle profile and revolve it
        let segments = 16;
        let mut points: Vec<Vec2> = (0..=segments)
            .map(|i| {
                let angle = (i as f32 / segments as f32) * std::f32::consts::PI;
                Vec2::new(angle.sin() * radius, angle.cos() * radius)
            })
            .collect();

        // Close the profile by adding center point
        points.push(Vec2::new(0.0, -radius));
        points.insert(0, Vec2::new(0.0, radius));

        let profile = Wire2D::new(points, true);

        // YZ plane: x_axis=Y, y_axis=Z
        self.revolve(
            &profile,
            center,
            Vec3::Y,
            Vec3::Z,
            &Axis3D::new(center, Vec3::Y),
            std::f32::consts::TAU,
        )
    }

    fn get_edges(&self, _solid: &Solid) -> CadResult<Vec<EdgeInfo>> {
        Err(CadError::OperationFailed(
            "Edge enumeration is not supported in Truck kernel".into(),
        ))
    }

    fn get_faces(&self, _solid: &Solid) -> CadResult<Vec<FaceInfo>> {
        Err(CadError::OperationFailed(
            "Face enumeration is not supported in Truck kernel".into(),
        ))
    }

    fn fillet(&self, _solid: &Solid, _edges: &[EdgeId], _radius: f32) -> CadResult<Solid> {
        Err(CadError::OperationFailed(
            "Fillet is not supported in Truck kernel".into(),
        ))
    }

    fn chamfer(&self, _solid: &Solid, _edges: &[EdgeId], _distance: f32) -> CadResult<Solid> {
        Err(CadError::OperationFailed(
            "Chamfer is not supported in Truck kernel".into(),
        ))
    }

    fn shell(
        &self,
        _solid: &Solid,
        _thickness: f32,
        _faces_to_remove: &[FaceId],
    ) -> CadResult<Solid> {
        Err(CadError::OperationFailed(
            "Shell is not supported in Truck kernel".into(),
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
        Err(CadError::OperationFailed(
            "Sweep is not supported in Truck kernel".into(),
        ))
    }

    fn loft(
        &self,
        _profiles: &[(Wire2D, Vec3, Vec3)],
        _create_solid: bool,
        _ruled: bool,
    ) -> CadResult<Solid> {
        Err(CadError::OperationFailed(
            "Loft is not supported in Truck kernel".into(),
        ))
    }

    fn import_step(
        &self,
        _path: &std::path::Path,
        _options: &StepImportOptions,
    ) -> CadResult<StepImportResult> {
        Err(CadError::OperationFailed(
            "STEP import is not supported in Truck kernel. Use OpenCASCADE kernel.".into(),
        ))
    }

    fn export_step(
        &self,
        _solid: &Solid,
        _path: &std::path::Path,
        _options: &StepExportOptions,
    ) -> CadResult<()> {
        Err(CadError::OperationFailed(
            "STEP export is not supported in Truck kernel. Use OpenCASCADE kernel.".into(),
        ))
    }

    fn export_step_multi(
        &self,
        _solids: &[&Solid],
        _path: &std::path::Path,
        _options: &StepExportOptions,
    ) -> CadResult<()> {
        Err(CadError::OperationFailed(
            "STEP export is not supported in Truck kernel. Use OpenCASCADE kernel.".into(),
        ))
    }
}
