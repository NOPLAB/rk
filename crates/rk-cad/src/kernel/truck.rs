//! Truck CAD Kernel Backend
//!
//! Pure Rust B-Rep kernel using the Truck library.
//!
//! Note: This implementation provides basic solid modeling capabilities.
//! Boolean operations and some features may require additional integration work.

use glam::{Vec2, Vec3};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

use truck_modeling::{InnerSpace, Point3, Solid as TruckSolid, Vector3, Vertex, Wire, builder};

use super::{Axis3D, BooleanType, CadError, CadKernel, CadResult, Solid, TessellatedMesh, Wire2D};

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
    fn points_to_3d(&self, points: &[Vec2], plane_origin: Vec3, plane_normal: Vec3) -> Vec<Point3> {
        // Calculate plane basis vectors
        let normal = Vector3::new(
            plane_normal.x as f64,
            plane_normal.y as f64,
            plane_normal.z as f64,
        );
        let origin = Point3::new(
            plane_origin.x as f64,
            plane_origin.y as f64,
            plane_origin.z as f64,
        );

        // Create basis vectors for the plane
        let up = if normal.z.abs() < 0.9 {
            Vector3::new(0.0, 0.0, 1.0)
        } else {
            Vector3::new(1.0, 0.0, 0.0)
        };
        let u = normal.cross(up).normalize();
        let v = normal.cross(u);

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
    fn create_wire(&self, profile: &Wire2D, plane_origin: Vec3, plane_normal: Vec3) -> Wire {
        let points_3d = self.points_to_3d(&profile.points, plane_origin, plane_normal);

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
        plane_normal: Vec3,
        direction: Vec3,
        distance: f32,
    ) -> CadResult<Solid> {
        if profile.points.len() < 3 {
            return Err(CadError::InvalidProfile(
                "Profile must have at least 3 points".into(),
            ));
        }

        // Create wire from profile
        let wire = self.create_wire(profile, plane_origin, plane_normal);

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
        plane_normal: Vec3,
        axis: &Axis3D,
        angle: f32,
    ) -> CadResult<Solid> {
        if profile.points.len() < 3 {
            return Err(CadError::InvalidProfile(
                "Profile must have at least 3 points".into(),
            ));
        }

        // Create wire from profile
        let wire = self.create_wire(profile, plane_origin, plane_normal);

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

    fn boolean(&self, _a: &Solid, _b: &Solid, _op: BooleanType) -> CadResult<Solid> {
        // Boolean operations require truck-shapeops which is not yet stable
        Err(CadError::OperationFailed(
            "Boolean operations are not yet implemented for Truck kernel".into(),
        ))
    }

    fn tessellate(&self, solid: &Solid, _tolerance: f32) -> CadResult<TessellatedMesh> {
        // Verify the solid exists
        let solids = self.solids.lock().unwrap();
        if !solids.contains_key(&solid.id) {
            return Err(CadError::TessellationFailed("Solid not found".into()));
        }

        // Note: Tessellation requires additional integration work due to trait bound
        // requirements in truck-meshalgo. The MeshableShape trait implementation
        // requires specific curve/surface types that need version alignment.
        //
        // For now, return an error indicating this feature needs implementation.
        // The solid modeling operations (extrude, revolve, primitives) work correctly.
        Err(CadError::TessellationFailed(
            "Tessellation requires additional integration work for Truck kernel. \
             Use OpenCASCADE kernel for full tessellation support."
                .into(),
        ))
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

        self.extrude(&wire, base_center, axis_normalized, axis_normalized, height)
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

        self.revolve(
            &profile,
            center,
            Vec3::X,
            &Axis3D::new(center, Vec3::Y),
            std::f32::consts::TAU,
        )
    }
}
