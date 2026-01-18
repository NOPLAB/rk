//! OpenCASCADE CAD Kernel Backend
//!
//! Provides bindings to the OpenCASCADE geometry kernel via opencascade-sys.

use glam::{Vec2, Vec3};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

use super::{
    Axis3D, BooleanType, CadError, CadKernel, CadResult, EdgeId, EdgeInfo, FaceId, FaceInfo, Solid,
    TessellatedMesh, Wire2D,
};

// Re-export OpenCASCADE types
use opencascade_sys::ffi;

/// OpenCASCADE-based CAD kernel
pub struct OpenCascadeKernel {
    /// Storage for solid data (keyed by UUID)
    solids: Mutex<HashMap<Uuid, OccSolid>>,
}

/// Wrapper for OpenCASCADE solid
struct OccSolid {
    shape: cxx::UniquePtr<ffi::TopoDS_Shape>,
}

impl Clone for OccSolid {
    fn clone(&self) -> Self {
        // Use MakeShape to clone
        Self {
            shape: ffi::BRepBuilderAPI_Copy_ctor(&self.shape).Shape(),
        }
    }
}

impl OpenCascadeKernel {
    /// Create a new OpenCASCADE kernel
    pub fn new() -> Self {
        Self {
            solids: Mutex::new(HashMap::new()),
        }
    }

    /// Store a solid and return a Solid reference
    fn store_solid(&self, shape: cxx::UniquePtr<ffi::TopoDS_Shape>) -> Solid {
        let id = Uuid::new_v4();
        let mut solids = self.solids.lock().unwrap();
        solids.insert(id, OccSolid { shape });
        Solid::new(id).with_kernel_data()
    }

    /// Get a stored solid by ID
    fn get_solid(&self, id: Uuid) -> Option<OccSolid> {
        let solids = self.solids.lock().unwrap();
        solids.get(&id).cloned()
    }

    /// Convert a Wire2D to OpenCASCADE wire
    fn create_wire(
        &self,
        profile: &Wire2D,
        plane_origin: Vec3,
        plane_normal: Vec3,
    ) -> CadResult<cxx::UniquePtr<ffi::TopoDS_Wire>> {
        // Calculate plane basis vectors
        let normal = ffi::new_gp_Dir(
            plane_normal.x as f64,
            plane_normal.y as f64,
            plane_normal.z as f64,
        );
        let origin = ffi::new_gp_Pnt(
            plane_origin.x as f64,
            plane_origin.y as f64,
            plane_origin.z as f64,
        );

        // Create basis vectors for the plane
        let up = if plane_normal.z.abs() < 0.9 {
            ffi::new_gp_Dir(0.0, 0.0, 1.0)
        } else {
            ffi::new_gp_Dir(1.0, 0.0, 0.0)
        };

        let u = ffi::gp_Dir_Crossed(&normal, &up);
        let v = ffi::gp_Dir_Crossed(&normal, &u);

        // Build wire from edges
        let mut wire_builder = ffi::BRepBuilderAPI_MakeWire_ctor();

        let points: Vec<_> = profile
            .points
            .iter()
            .map(|p| {
                let x = p.x as f64;
                let y = p.y as f64;

                // Transform 2D point to 3D
                let px = ffi::gp_Pnt_X(&origin) + ffi::gp_Dir_X(&u) * x + ffi::gp_Dir_X(&v) * y;
                let py = ffi::gp_Pnt_Y(&origin) + ffi::gp_Dir_Y(&u) * x + ffi::gp_Dir_Y(&v) * y;
                let pz = ffi::gp_Pnt_Z(&origin) + ffi::gp_Dir_Z(&u) * x + ffi::gp_Dir_Z(&v) * y;

                ffi::new_gp_Pnt(px, py, pz)
            })
            .collect();

        // Create edges between consecutive points
        for i in 0..points.len() {
            let p1 = &points[i];
            let p2 = &points[(i + 1) % points.len()];

            let edge = ffi::BRepBuilderAPI_MakeEdge_gp_Pnt_gp_Pnt(p1, p2);
            ffi::BRepBuilderAPI_MakeWire_Add_edge(&mut wire_builder, &edge.Edge());
        }

        Ok(wire_builder.Wire())
    }
}

impl Default for OpenCascadeKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl CadKernel for OpenCascadeKernel {
    fn name(&self) -> &str {
        "opencascade"
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
        let wire = self.create_wire(profile, plane_origin, plane_normal)?;

        // Create face from wire
        let face = ffi::BRepBuilderAPI_MakeFace_wire(&wire, true);

        // Create extrusion direction vector
        let dir = ffi::new_gp_Vec(
            direction.x as f64 * distance as f64,
            direction.y as f64 * distance as f64,
            direction.z as f64 * distance as f64,
        );

        // Extrude
        let prism = ffi::BRepPrimAPI_MakePrism_ctor(&face.Face().as_shape(), &dir, false, true);

        Ok(self.store_solid(prism.Shape()))
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
        let wire = self.create_wire(profile, plane_origin, plane_normal)?;

        // Create face from wire
        let face = ffi::BRepBuilderAPI_MakeFace_wire(&wire, true);

        // Create rotation axis
        let axis_origin = ffi::new_gp_Pnt(
            axis.origin.x as f64,
            axis.origin.y as f64,
            axis.origin.z as f64,
        );
        let axis_dir = ffi::new_gp_Dir(
            axis.direction.x as f64,
            axis.direction.y as f64,
            axis.direction.z as f64,
        );
        let gp_axis = ffi::new_gp_Ax1(&axis_origin, &axis_dir);

        // Revolve
        let revol =
            ffi::BRepPrimAPI_MakeRevol_ctor(&face.Face().as_shape(), &gp_axis, angle as f64, true);

        Ok(self.store_solid(revol.Shape()))
    }

    fn boolean(&self, a: &Solid, b: &Solid, op: BooleanType) -> CadResult<Solid> {
        let solid_a = self
            .get_solid(a.id)
            .ok_or_else(|| CadError::OperationFailed("First solid not found".into()))?;

        let solid_b = self
            .get_solid(b.id)
            .ok_or_else(|| CadError::OperationFailed("Second solid not found".into()))?;

        let result = match op {
            BooleanType::Union => ffi::BRepAlgoAPI_Fuse_ctor(&solid_a.shape, &solid_b.shape),
            BooleanType::Subtract => ffi::BRepAlgoAPI_Cut_ctor(&solid_a.shape, &solid_b.shape),
            BooleanType::Intersect => ffi::BRepAlgoAPI_Common_ctor(&solid_a.shape, &solid_b.shape),
        };

        Ok(self.store_solid(result.Shape()))
    }

    fn tessellate(&self, solid: &Solid, tolerance: f32) -> CadResult<TessellatedMesh> {
        let occ_solid = self
            .get_solid(solid.id)
            .ok_or_else(|| CadError::TessellationFailed("Solid not found".into()))?;

        // Create mesh
        let mut mesh_builder = ffi::BRepMesh_IncrementalMesh_ctor(
            &occ_solid.shape,
            tolerance as f64,
            false,
            0.5,
            true,
        );

        let mut result = TessellatedMesh::new();

        // Extract triangulation from each face
        let mut explorer =
            ffi::TopExp_Explorer_ctor(&occ_solid.shape, ffi::TopAbs_ShapeEnum::TopAbs_FACE);

        while ffi::TopExp_Explorer_More(&explorer) {
            let face_shape = ffi::TopExp_Explorer_Current(&explorer);
            let face = ffi::TopoDS_cast_to_face(&face_shape);

            let location = ffi::TopLoc_Location_ctor();
            let triangulation = ffi::BRep_Tool_Triangulation(&face, &location);

            if !triangulation.is_null() {
                let nb_nodes = ffi::Poly_Triangulation_NbNodes(&triangulation);
                let nb_triangles = ffi::Poly_Triangulation_NbTriangles(&triangulation);

                let vertex_offset = result.vertices.len() as u32;

                // Extract vertices
                for i in 1..=nb_nodes {
                    let node = ffi::Poly_Triangulation_Node(&triangulation, i);
                    let transformed = ffi::gp_Pnt_Transformed(
                        &node,
                        &ffi::TopLoc_Location_Transformation(&location),
                    );
                    result.vertices.push([
                        ffi::gp_Pnt_X(&transformed) as f32,
                        ffi::gp_Pnt_Y(&transformed) as f32,
                        ffi::gp_Pnt_Z(&transformed) as f32,
                    ]);
                    // Placeholder normals
                    result.normals.push([0.0, 1.0, 0.0]);
                }

                // Extract triangles
                for i in 1..=nb_triangles {
                    let triangle = ffi::Poly_Triangulation_Triangle(&triangulation, i);
                    let (n1, n2, n3) = (
                        ffi::Poly_Triangle_Value(&triangle, 1) as u32 - 1 + vertex_offset,
                        ffi::Poly_Triangle_Value(&triangle, 2) as u32 - 1 + vertex_offset,
                        ffi::Poly_Triangle_Value(&triangle, 3) as u32 - 1 + vertex_offset,
                    );

                    // Check face orientation
                    let orientation = ffi::TopoDS_Shape_Orientation(&face_shape);
                    if orientation == ffi::TopAbs_Orientation::TopAbs_REVERSED {
                        result.indices.push(n1);
                        result.indices.push(n3);
                        result.indices.push(n2);
                    } else {
                        result.indices.push(n1);
                        result.indices.push(n2);
                        result.indices.push(n3);
                    }
                }
            }

            ffi::TopExp_Explorer_Next(&mut explorer);
        }

        // Compute normals from triangles
        self.compute_normals(&mut result);

        Ok(result)
    }

    fn create_box(&self, center: Vec3, size: Vec3) -> CadResult<Solid> {
        let half = size * 0.5;
        let min = center - half;
        let max = center + half;

        let p1 = ffi::new_gp_Pnt(min.x as f64, min.y as f64, min.z as f64);
        let p2 = ffi::new_gp_Pnt(max.x as f64, max.y as f64, max.z as f64);

        let box_maker = ffi::BRepPrimAPI_MakeBox_ctor(&p1, &p2);
        Ok(self.store_solid(box_maker.Shape()))
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

        let origin = ffi::new_gp_Pnt(
            base_center.x as f64,
            base_center.y as f64,
            base_center.z as f64,
        );
        let dir = ffi::new_gp_Dir(
            axis_normalized.x as f64,
            axis_normalized.y as f64,
            axis_normalized.z as f64,
        );
        let ax2 = ffi::new_gp_Ax2(&origin, &dir);

        let cylinder = ffi::BRepPrimAPI_MakeCylinder_ctor(&ax2, radius as f64, height as f64);
        Ok(self.store_solid(cylinder.Shape()))
    }

    fn create_sphere(&self, center: Vec3, radius: f32) -> CadResult<Solid> {
        let origin = ffi::new_gp_Pnt(center.x as f64, center.y as f64, center.z as f64);

        let sphere = ffi::BRepPrimAPI_MakeSphere_ctor(&origin, radius as f64);
        Ok(self.store_solid(sphere.Shape()))
    }

    fn get_edges(&self, solid: &Solid) -> CadResult<Vec<EdgeInfo>> {
        let occ_solid = self
            .get_solid(solid.id)
            .ok_or_else(|| CadError::OperationFailed("Solid not found".into()))?;

        let mut edges = Vec::new();
        let mut index = 0u32;

        // Use TopExp_Explorer to iterate through all edges
        let mut explorer =
            ffi::TopExp_Explorer_ctor(&occ_solid.shape, ffi::TopAbs_ShapeEnum::TopAbs_EDGE);

        while ffi::TopExp_Explorer_More(&explorer) {
            let edge_shape = ffi::TopExp_Explorer_Current(&explorer);
            let edge = ffi::TopoDS_cast_to_edge(&edge_shape);

            // Get edge curve and parameters
            let mut first = 0.0f64;
            let mut last = 0.0f64;
            let curve = ffi::BRep_Tool_Curve(&edge, &mut first, &mut last);

            if !curve.is_null() {
                // Get start and end points
                let start_pnt = ffi::Geom_Curve_Value(&curve, first);
                let end_pnt = ffi::Geom_Curve_Value(&curve, last);
                let mid_pnt = ffi::Geom_Curve_Value(&curve, (first + last) / 2.0);

                let start = Vec3::new(
                    ffi::gp_Pnt_X(&start_pnt) as f32,
                    ffi::gp_Pnt_Y(&start_pnt) as f32,
                    ffi::gp_Pnt_Z(&start_pnt) as f32,
                );
                let end = Vec3::new(
                    ffi::gp_Pnt_X(&end_pnt) as f32,
                    ffi::gp_Pnt_Y(&end_pnt) as f32,
                    ffi::gp_Pnt_Z(&end_pnt) as f32,
                );
                let midpoint = Vec3::new(
                    ffi::gp_Pnt_X(&mid_pnt) as f32,
                    ffi::gp_Pnt_Y(&mid_pnt) as f32,
                    ffi::gp_Pnt_Z(&mid_pnt) as f32,
                );

                edges.push(EdgeInfo {
                    id: EdgeId::new(solid.id, index),
                    start,
                    end,
                    midpoint,
                    length: (end - start).length(),
                });
            }

            index += 1;
            ffi::TopExp_Explorer_Next(&mut explorer);
        }

        Ok(edges)
    }

    fn get_faces(&self, solid: &Solid) -> CadResult<Vec<FaceInfo>> {
        let occ_solid = self
            .get_solid(solid.id)
            .ok_or_else(|| CadError::OperationFailed("Solid not found".into()))?;

        let mut faces = Vec::new();
        let mut index = 0u32;

        // Use TopExp_Explorer to iterate through all faces
        let mut explorer =
            ffi::TopExp_Explorer_ctor(&occ_solid.shape, ffi::TopAbs_ShapeEnum::TopAbs_FACE);

        while ffi::TopExp_Explorer_More(&explorer) {
            let face_shape = ffi::TopExp_Explorer_Current(&explorer);
            let face = ffi::TopoDS_cast_to_face(&face_shape);

            // Get surface
            let surface = ffi::BRep_Tool_Surface(&face);

            if !surface.is_null() {
                // Get face bounds
                let mut umin = 0.0f64;
                let mut umax = 0.0f64;
                let mut vmin = 0.0f64;
                let mut vmax = 0.0f64;
                ffi::BRepTools_UVBounds_face(&face, &mut umin, &mut umax, &mut vmin, &mut vmax);

                // Calculate center point
                let u_mid = (umin + umax) / 2.0;
                let v_mid = (vmin + vmax) / 2.0;

                let center_pnt = ffi::Geom_Surface_Value(&surface, u_mid, v_mid);
                let center = Vec3::new(
                    ffi::gp_Pnt_X(&center_pnt) as f32,
                    ffi::gp_Pnt_Y(&center_pnt) as f32,
                    ffi::gp_Pnt_Z(&center_pnt) as f32,
                );

                // Get normal at center
                let mut pnt = ffi::new_gp_Pnt(0.0, 0.0, 0.0);
                let mut normal_vec = ffi::new_gp_Vec(0.0, 0.0, 1.0);
                ffi::BRepGProp_Face_Normal(&face, u_mid, v_mid, &mut pnt, &mut normal_vec);

                let normal = Vec3::new(
                    ffi::gp_Vec_X(&normal_vec) as f32,
                    ffi::gp_Vec_Y(&normal_vec) as f32,
                    ffi::gp_Vec_Z(&normal_vec) as f32,
                );

                // Calculate approximate area
                let mut props = ffi::GProp_GProps_ctor();
                ffi::BRepGProp_SurfaceProperties(&face_shape, &mut props, 1e-6);
                let area = ffi::GProp_GProps_Mass(&props) as f32;

                faces.push(FaceInfo {
                    id: FaceId::new(solid.id, index),
                    center,
                    normal: normal.normalize(),
                    area,
                });
            }

            index += 1;
            ffi::TopExp_Explorer_Next(&mut explorer);
        }

        Ok(faces)
    }

    fn fillet(&self, solid: &Solid, edges: &[EdgeId], radius: f32) -> CadResult<Solid> {
        let occ_solid = self
            .get_solid(solid.id)
            .ok_or_else(|| CadError::OperationFailed("Solid not found".into()))?;

        if edges.is_empty() {
            return Err(CadError::OperationFailed("No edges specified".into()));
        }

        // Create fillet maker
        let mut fillet = ffi::BRepFilletAPI_MakeFillet_ctor(&occ_solid.shape);

        // Find and add edges by index
        let mut edge_index = 0u32;
        let mut explorer =
            ffi::TopExp_Explorer_ctor(&occ_solid.shape, ffi::TopAbs_ShapeEnum::TopAbs_EDGE);

        while ffi::TopExp_Explorer_More(&explorer) {
            // Check if this edge is in our list
            if edges.iter().any(|e| e.index == edge_index) {
                let edge_shape = ffi::TopExp_Explorer_Current(&explorer);
                let edge = ffi::TopoDS_cast_to_edge(&edge_shape);
                ffi::BRepFilletAPI_MakeFillet_Add(&mut fillet, radius as f64, &edge);
            }

            edge_index += 1;
            ffi::TopExp_Explorer_Next(&mut explorer);
        }

        // Build the result
        ffi::BRepFilletAPI_MakeFillet_Build(&mut fillet);

        let result = ffi::BRepFilletAPI_MakeFillet_Shape(&fillet);
        Ok(self.store_solid(result))
    }

    fn chamfer(&self, solid: &Solid, edges: &[EdgeId], distance: f32) -> CadResult<Solid> {
        let occ_solid = self
            .get_solid(solid.id)
            .ok_or_else(|| CadError::OperationFailed("Solid not found".into()))?;

        if edges.is_empty() {
            return Err(CadError::OperationFailed("No edges specified".into()));
        }

        // Create chamfer maker
        let mut chamfer = ffi::BRepFilletAPI_MakeChamfer_ctor(&occ_solid.shape);

        // Build edge-face map
        let mut edge_face_map = ffi::TopTools_IndexedDataMapOfShapeListOfShape_ctor();
        ffi::TopExp_MapShapesAndAncestors(
            &occ_solid.shape,
            ffi::TopAbs_ShapeEnum::TopAbs_EDGE,
            ffi::TopAbs_ShapeEnum::TopAbs_FACE,
            &mut edge_face_map,
        );

        // Find and add edges by index
        let mut edge_index = 0u32;
        let mut explorer =
            ffi::TopExp_Explorer_ctor(&occ_solid.shape, ffi::TopAbs_ShapeEnum::TopAbs_EDGE);

        while ffi::TopExp_Explorer_More(&explorer) {
            // Check if this edge is in our list
            if edges.iter().any(|e| e.index == edge_index) {
                let edge_shape = ffi::TopExp_Explorer_Current(&explorer);
                let edge = ffi::TopoDS_cast_to_edge(&edge_shape);

                // Get adjacent face
                let face_list = ffi::TopTools_IndexedDataMapOfShapeListOfShape_FindFromKey(
                    &edge_face_map,
                    &edge_shape,
                );
                if !ffi::TopTools_ListOfShape_IsEmpty(&face_list) {
                    let face_shape = ffi::TopTools_ListOfShape_First(&face_list);
                    let face = ffi::TopoDS_cast_to_face(face_shape);
                    ffi::BRepFilletAPI_MakeChamfer_Add_face(
                        &mut chamfer,
                        distance as f64,
                        &edge,
                        &face,
                    );
                }
            }

            edge_index += 1;
            ffi::TopExp_Explorer_Next(&mut explorer);
        }

        // Build the result
        ffi::BRepFilletAPI_MakeChamfer_Build(&mut chamfer);

        let result = ffi::BRepFilletAPI_MakeChamfer_Shape(&chamfer);
        Ok(self.store_solid(result))
    }

    fn shell(&self, solid: &Solid, thickness: f32, faces_to_remove: &[FaceId]) -> CadResult<Solid> {
        let occ_solid = self
            .get_solid(solid.id)
            .ok_or_else(|| CadError::OperationFailed("Solid not found".into()))?;

        // Collect faces to remove
        let mut faces_list = ffi::TopTools_ListOfShape_ctor();

        if !faces_to_remove.is_empty() {
            let mut face_index = 0u32;
            let mut explorer =
                ffi::TopExp_Explorer_ctor(&occ_solid.shape, ffi::TopAbs_ShapeEnum::TopAbs_FACE);

            while ffi::TopExp_Explorer_More(&explorer) {
                if faces_to_remove.iter().any(|f| f.index == face_index) {
                    let face_shape = ffi::TopExp_Explorer_Current(&explorer);
                    ffi::TopTools_ListOfShape_Append(&mut faces_list, &face_shape);
                }

                face_index += 1;
                ffi::TopExp_Explorer_Next(&mut explorer);
            }
        }

        // Create thick solid (shell)
        let thick_solid = ffi::BRepOffsetAPI_MakeThickSolid_ctor();
        ffi::BRepOffsetAPI_MakeThickSolid_MakeThickSolidByJoin(
            &thick_solid,
            &occ_solid.shape,
            &faces_list,
            thickness as f64,
            1e-6, // tolerance
        );

        let result = ffi::BRepOffsetAPI_MakeThickSolid_Shape(&thick_solid);
        Ok(self.store_solid(result))
    }

    fn sweep(
        &self,
        profile: &Wire2D,
        profile_plane_origin: Vec3,
        profile_plane_normal: Vec3,
        path: &Wire2D,
        path_plane_origin: Vec3,
        path_plane_normal: Vec3,
    ) -> CadResult<Solid> {
        if profile.points.len() < 3 {
            return Err(CadError::InvalidProfile(
                "Profile must have at least 3 points".into(),
            ));
        }
        if path.points.len() < 2 {
            return Err(CadError::InvalidProfile(
                "Path must have at least 2 points".into(),
            ));
        }

        // Create profile wire
        let profile_wire = self.create_wire(profile, profile_plane_origin, profile_plane_normal)?;

        // Create path wire (spine)
        let path_wire = self.create_wire(path, path_plane_origin, path_plane_normal)?;

        // Create pipe shell
        let mut pipe = ffi::BRepOffsetAPI_MakePipeShell_ctor(&path_wire);

        // Add profile
        ffi::BRepOffsetAPI_MakePipeShell_Add_wire(&mut pipe, &profile_wire, false, false);

        // Build
        ffi::BRepOffsetAPI_MakePipeShell_Build(&mut pipe);

        // Make solid
        ffi::BRepOffsetAPI_MakePipeShell_MakeSolid(&mut pipe);

        let result = ffi::BRepOffsetAPI_MakePipeShell_Shape(&pipe);
        Ok(self.store_solid(result))
    }

    fn loft(
        &self,
        profiles: &[(Wire2D, Vec3, Vec3)],
        create_solid: bool,
        ruled: bool,
    ) -> CadResult<Solid> {
        if profiles.len() < 2 {
            return Err(CadError::InvalidProfile(
                "Loft requires at least 2 profiles".into(),
            ));
        }

        // Create loft maker
        let mut loft = ffi::BRepOffsetAPI_ThruSections_ctor(create_solid, ruled, 1e-6);

        // Add all profiles
        for (profile, origin, normal) in profiles {
            if profile.points.len() < 3 {
                return Err(CadError::InvalidProfile(
                    "Each profile must have at least 3 points".into(),
                ));
            }

            let wire = self.create_wire(profile, *origin, *normal)?;
            ffi::BRepOffsetAPI_ThruSections_AddWire(&mut loft, &wire);
        }

        // Build
        ffi::BRepOffsetAPI_ThruSections_Build(&mut loft);

        let result = ffi::BRepOffsetAPI_ThruSections_Shape(&loft);
        Ok(self.store_solid(result))
    }
}

impl OpenCascadeKernel {
    /// Compute normals for a tessellated mesh
    fn compute_normals(&self, mesh: &mut TessellatedMesh) {
        // Initialize normals to zero
        for normal in mesh.normals.iter_mut() {
            *normal = [0.0, 0.0, 0.0];
        }

        // Accumulate face normals
        for chunk in mesh.indices.chunks(3) {
            if chunk.len() != 3 {
                continue;
            }
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;
            let i2 = chunk[2] as usize;

            let v0 = Vec3::from(mesh.vertices[i0]);
            let v1 = Vec3::from(mesh.vertices[i1]);
            let v2 = Vec3::from(mesh.vertices[i2]);

            let e1 = v1 - v0;
            let e2 = v2 - v0;
            let face_normal = e1.cross(e2);

            // Add to each vertex
            for &i in &[i0, i1, i2] {
                mesh.normals[i][0] += face_normal.x;
                mesh.normals[i][1] += face_normal.y;
                mesh.normals[i][2] += face_normal.z;
            }
        }

        // Normalize
        for normal in mesh.normals.iter_mut() {
            let n = Vec3::from(*normal);
            let len = n.length();
            if len > 1e-6 {
                let normalized = n / len;
                *normal = [normalized.x, normalized.y, normalized.z];
            } else {
                *normal = [0.0, 1.0, 0.0];
            }
        }
    }
}
