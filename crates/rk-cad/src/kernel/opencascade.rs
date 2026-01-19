//! OpenCASCADE CAD Kernel Backend
//!
//! Provides bindings to the OpenCASCADE geometry kernel via opencascade-sys.

use glam::Vec3;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

use super::{
    Axis3D, BooleanType, CadError, CadKernel, CadResult, EdgeId, EdgeInfo, FaceId, FaceInfo, Solid,
    StepExportOptions, StepImportOptions, StepImportResult, TessellatedMesh, Wire2D,
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

unsafe impl Send for OccSolid {}
unsafe impl Sync for OccSolid {}

impl Clone for OccSolid {
    fn clone(&self) -> Self {
        // Use TopoDS_Shape_to_owned to clone
        Self {
            shape: ffi::TopoDS_Shape_to_owned(&self.shape),
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
        plane_x_axis: Vec3,
        plane_y_axis: Vec3,
    ) -> CadResult<cxx::UniquePtr<ffi::TopoDS_Wire>> {
        let origin = ffi::new_point(
            plane_origin.x as f64,
            plane_origin.y as f64,
            plane_origin.z as f64,
        );

        // Use the provided x_axis and y_axis for the plane
        let ux = plane_x_axis.x as f64;
        let uy = plane_x_axis.y as f64;
        let uz = plane_x_axis.z as f64;

        let vx = plane_y_axis.x as f64;
        let vy = plane_y_axis.y as f64;
        let vz = plane_y_axis.z as f64;

        // Build wire from edges
        let mut wire_builder = ffi::BRepBuilderAPI_MakeWire_ctor();

        let ox = origin.X();
        let oy = origin.Y();
        let oz = origin.Z();

        let points: Vec<_> = profile
            .points
            .iter()
            .map(|p| {
                let x = p.x as f64;
                let y = p.y as f64;

                // Transform 2D point to 3D
                let px = ox + ux * x + vx * y;
                let py = oy + uy * x + vy * y;
                let pz = oz + uz * x + vz * y;

                ffi::new_point(px, py, pz)
            })
            .collect();

        // Create edges between consecutive points
        for i in 0..points.len() {
            let p1 = &points[i];
            let p2 = &points[(i + 1) % points.len()];

            let mut edge_maker = ffi::BRepBuilderAPI_MakeEdge_gp_Pnt_gp_Pnt(p1, p2);
            let edge = ffi::TopoDS_Edge_to_owned(edge_maker.pin_mut().Edge());
            wire_builder.pin_mut().add_edge(&edge);
        }

        Ok(ffi::TopoDS_Wire_to_owned(wire_builder.pin_mut().Wire()))
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
        let wire = self.create_wire(profile, plane_origin, plane_x_axis, plane_y_axis)?;

        // Create face from wire
        let face_maker = ffi::BRepBuilderAPI_MakeFace_wire(&wire, true);

        // Create extrusion direction vector
        let dir = ffi::new_vec(
            direction.x as f64 * distance as f64,
            direction.y as f64 * distance as f64,
            direction.z as f64 * distance as f64,
        );

        // Extrude
        let mut prism = ffi::BRepPrimAPI_MakePrism_ctor(
            ffi::cast_face_to_shape(face_maker.Face()),
            &dir,
            false,
            true,
        );

        Ok(self.store_solid(ffi::TopoDS_Shape_to_owned(prism.pin_mut().Shape())))
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
        let wire = self.create_wire(profile, plane_origin, plane_x_axis, plane_y_axis)?;

        // Create face from wire
        let face_maker = ffi::BRepBuilderAPI_MakeFace_wire(&wire, true);

        // Create rotation axis
        let axis_origin = ffi::new_point(
            axis.origin.x as f64,
            axis.origin.y as f64,
            axis.origin.z as f64,
        );
        let axis_dir = ffi::gp_Dir_ctor(
            axis.direction.x as f64,
            axis.direction.y as f64,
            axis.direction.z as f64,
        );
        let gp_axis = ffi::gp_Ax1_ctor(&axis_origin, &axis_dir);

        // Revolve
        let mut revol = ffi::BRepPrimAPI_MakeRevol_ctor(
            ffi::cast_face_to_shape(face_maker.Face()),
            &gp_axis,
            angle as f64,
            true,
        );

        Ok(self.store_solid(ffi::TopoDS_Shape_to_owned(revol.pin_mut().Shape())))
    }

    fn boolean(&self, a: &Solid, b: &Solid, op: BooleanType) -> CadResult<Solid> {
        let solid_a = self
            .get_solid(a.id)
            .ok_or_else(|| CadError::OperationFailed("First solid not found".into()))?;

        let solid_b = self
            .get_solid(b.id)
            .ok_or_else(|| CadError::OperationFailed("Second solid not found".into()))?;

        let result_shape = match op {
            BooleanType::Union => {
                let mut fuse = ffi::BRepAlgoAPI_Fuse_ctor(&solid_a.shape, &solid_b.shape);
                ffi::TopoDS_Shape_to_owned(fuse.pin_mut().Shape())
            }
            BooleanType::Subtract => {
                let mut cut = ffi::BRepAlgoAPI_Cut_ctor(&solid_a.shape, &solid_b.shape);
                ffi::TopoDS_Shape_to_owned(cut.pin_mut().Shape())
            }
            BooleanType::Intersect => {
                let mut common = ffi::BRepAlgoAPI_Common_ctor(&solid_a.shape, &solid_b.shape);
                ffi::TopoDS_Shape_to_owned(common.pin_mut().Shape())
            }
        };

        Ok(self.store_solid(result_shape))
    }

    fn tessellate(&self, solid: &Solid, tolerance: f32) -> CadResult<TessellatedMesh> {
        let occ_solid = self
            .get_solid(solid.id)
            .ok_or_else(|| CadError::TessellationFailed("Solid not found".into()))?;

        // Create mesh
        let _mesh_builder = ffi::BRepMesh_IncrementalMesh_ctor(&occ_solid.shape, tolerance as f64);

        let mut result = TessellatedMesh::new();

        // Extract triangulation from each face
        let mut explorer =
            ffi::TopExp_Explorer_ctor(&occ_solid.shape, ffi::TopAbs_ShapeEnum::TopAbs_FACE);

        while explorer.More() {
            let face_shape = explorer.Current();
            let face = ffi::TopoDS_cast_to_face(face_shape);

            let mut location = ffi::TopLoc_Location_ctor();
            let triangulation = ffi::BRep_Tool_Triangulation(face, location.pin_mut());

            if !triangulation.IsNull() {
                let tri = ffi::Handle_Poly_Triangulation_Get(&triangulation)
                    .map_err(|e| CadError::TessellationFailed(e.to_string()))?;

                let nb_nodes = tri.NbNodes();
                let nb_triangles = tri.NbTriangles();

                let vertex_offset = result.vertices.len() as u32;

                // Get the transformation from location
                let transform = ffi::TopLoc_Location_Transformation(&location);

                // Extract vertices
                for i in 1..=nb_nodes {
                    let mut node = ffi::Poly_Triangulation_Node(tri, i);
                    node.pin_mut().Transform(&transform);
                    result
                        .vertices
                        .push([node.X() as f32, node.Y() as f32, node.Z() as f32]);
                    // Placeholder normals
                    result.normals.push([0.0, 1.0, 0.0]);
                }

                // Extract triangles
                for i in 1..=nb_triangles {
                    let triangle = tri.Triangle(i);
                    let (n1, n2, n3) = (
                        triangle.Value(1) as u32 - 1 + vertex_offset,
                        triangle.Value(2) as u32 - 1 + vertex_offset,
                        triangle.Value(3) as u32 - 1 + vertex_offset,
                    );

                    // Check face orientation
                    let orientation = face_shape.Orientation();
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

            explorer.pin_mut().Next();
        }

        // Compute normals from triangles
        self.compute_normals(&mut result);

        Ok(result)
    }

    fn create_box(&self, center: Vec3, size: Vec3) -> CadResult<Solid> {
        let half = size * 0.5;
        let min = center - half;

        let p1 = ffi::new_point(min.x as f64, min.y as f64, min.z as f64);

        let mut box_maker =
            ffi::BRepPrimAPI_MakeBox_ctor(&p1, size.x as f64, size.y as f64, size.z as f64);
        Ok(self.store_solid(ffi::TopoDS_Shape_to_owned(box_maker.pin_mut().Shape())))
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

        let origin = ffi::new_point(
            base_center.x as f64,
            base_center.y as f64,
            base_center.z as f64,
        );
        let dir = ffi::gp_Dir_ctor(
            axis_normalized.x as f64,
            axis_normalized.y as f64,
            axis_normalized.z as f64,
        );
        let ax2 = ffi::gp_Ax2_ctor(&origin, &dir);

        let mut cylinder = ffi::BRepPrimAPI_MakeCylinder_ctor(&ax2, radius as f64, height as f64);
        Ok(self.store_solid(ffi::TopoDS_Shape_to_owned(cylinder.pin_mut().Shape())))
    }

    fn create_sphere(&self, center: Vec3, radius: f32) -> CadResult<Solid> {
        // Create sphere at origin then translate
        let mut sphere = ffi::BRepPrimAPI_MakeSphere_ctor(radius as f64);
        let sphere_shape = ffi::TopoDS_Shape_to_owned(sphere.pin_mut().Shape());

        // Create translation transform
        let mut transform = ffi::new_transform();
        let origin = ffi::new_point(0.0, 0.0, 0.0);
        let target = ffi::new_point(center.x as f64, center.y as f64, center.z as f64);
        transform.pin_mut().SetTranslation(&origin, &target);

        let mut transformed = ffi::BRepBuilderAPI_Transform_ctor(&sphere_shape, &transform, true);
        Ok(self.store_solid(ffi::TopoDS_Shape_to_owned(transformed.pin_mut().Shape())))
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

        while explorer.More() {
            let edge_shape = explorer.Current();
            let edge = ffi::TopoDS_cast_to_edge(edge_shape);

            // Get edge curve and parameters
            let mut first = 0.0f64;
            let mut last = 0.0f64;
            let curve = ffi::BRep_Tool_Curve(edge, &mut first, &mut last);

            if !curve.IsNull() {
                // Get start and end points
                let start_pnt = ffi::HandleGeomCurve_Value(&curve, first);
                let end_pnt = ffi::HandleGeomCurve_Value(&curve, last);
                let mid_pnt = ffi::HandleGeomCurve_Value(&curve, (first + last) / 2.0);

                let start = Vec3::new(
                    start_pnt.X() as f32,
                    start_pnt.Y() as f32,
                    start_pnt.Z() as f32,
                );
                let end = Vec3::new(end_pnt.X() as f32, end_pnt.Y() as f32, end_pnt.Z() as f32);
                let midpoint =
                    Vec3::new(mid_pnt.X() as f32, mid_pnt.Y() as f32, mid_pnt.Z() as f32);

                edges.push(EdgeInfo {
                    id: EdgeId::new(solid.id, index),
                    start,
                    end,
                    midpoint,
                    length: (end - start).length(),
                });
            }

            index += 1;
            explorer.pin_mut().Next();
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

        while explorer.More() {
            let face_shape = explorer.Current();
            let face = ffi::TopoDS_cast_to_face(face_shape);

            // Get surface
            let surface = ffi::BRep_Tool_Surface(face);

            if !surface.IsNull() {
                // Get face properties using BRepGProp_Face
                let brep_face = ffi::BRepGProp_Face_ctor(face);

                // Use center of parameter space (u=0.5, v=0.5) for now
                let mut center_pnt = ffi::new_point(0.0, 0.0, 0.0);
                let mut normal_vec = ffi::new_vec(0.0, 0.0, 1.0);
                brep_face.Normal(0.5, 0.5, center_pnt.pin_mut(), normal_vec.pin_mut());

                let center = Vec3::new(
                    center_pnt.X() as f32,
                    center_pnt.Y() as f32,
                    center_pnt.Z() as f32,
                );

                let normal = Vec3::new(
                    normal_vec.X() as f32,
                    normal_vec.Y() as f32,
                    normal_vec.Z() as f32,
                );

                // Calculate approximate area
                let mut props = ffi::GProp_GProps_ctor();
                ffi::BRepGProp_SurfaceProperties(face_shape, props.pin_mut());
                let area = props.Mass() as f32;

                faces.push(FaceInfo {
                    id: FaceId::new(solid.id, index),
                    center,
                    normal: normal.normalize(),
                    area,
                });
            }

            index += 1;
            explorer.pin_mut().Next();
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

        while explorer.More() {
            // Check if this edge is in our list
            if edges.iter().any(|e| e.index == edge_index) {
                let edge_shape = explorer.Current();
                let edge = ffi::TopoDS_cast_to_edge(edge_shape);
                fillet.pin_mut().add_edge(radius as f64, edge);
            }

            edge_index += 1;
            explorer.pin_mut().Next();
        }

        // Build the result
        let progress = ffi::Message_ProgressRange_ctor();
        fillet.pin_mut().Build(&progress);

        let result = ffi::TopoDS_Shape_to_owned(fillet.pin_mut().Shape());
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

        // Find and add edges by index
        let mut edge_index = 0u32;
        let mut explorer =
            ffi::TopExp_Explorer_ctor(&occ_solid.shape, ffi::TopAbs_ShapeEnum::TopAbs_EDGE);

        while explorer.More() {
            // Check if this edge is in our list
            if edges.iter().any(|e| e.index == edge_index) {
                let edge_shape = explorer.Current();
                let edge = ffi::TopoDS_cast_to_edge(edge_shape);
                chamfer.pin_mut().add_edge(distance as f64, edge);
            }

            edge_index += 1;
            explorer.pin_mut().Next();
        }

        // Build the result
        let progress = ffi::Message_ProgressRange_ctor();
        chamfer.pin_mut().Build(&progress);

        let result = ffi::TopoDS_Shape_to_owned(chamfer.pin_mut().Shape());
        Ok(self.store_solid(result))
    }

    fn shell(&self, solid: &Solid, thickness: f32, faces_to_remove: &[FaceId]) -> CadResult<Solid> {
        let occ_solid = self
            .get_solid(solid.id)
            .ok_or_else(|| CadError::OperationFailed("Solid not found".into()))?;

        // Collect faces to remove
        let mut faces_list = ffi::new_list_of_shape();

        if !faces_to_remove.is_empty() {
            let mut face_index = 0u32;
            let mut explorer =
                ffi::TopExp_Explorer_ctor(&occ_solid.shape, ffi::TopAbs_ShapeEnum::TopAbs_FACE);

            while explorer.More() {
                if faces_to_remove.iter().any(|f| f.index == face_index) {
                    let face_shape = explorer.Current();
                    let face = ffi::TopoDS_cast_to_face(face_shape);
                    ffi::shape_list_append_face(faces_list.pin_mut(), face);
                }

                face_index += 1;
                explorer.pin_mut().Next();
            }
        }

        // Create thick solid (shell)
        let mut thick_solid = ffi::BRepOffsetAPI_MakeThickSolid_ctor();
        ffi::MakeThickSolidByJoin(
            thick_solid.pin_mut(),
            &occ_solid.shape,
            &faces_list,
            thickness as f64,
            1e-6, // tolerance
        );

        let progress = ffi::Message_ProgressRange_ctor();
        thick_solid.pin_mut().Build(&progress);

        let result = ffi::TopoDS_Shape_to_owned(thick_solid.pin_mut().Shape());
        Ok(self.store_solid(result))
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
        // BRepOffsetAPI_MakePipeShell is not exposed in opencascade-sys 0.2.0
        Err(CadError::OperationFailed(
            "Sweep operation not supported in opencascade-sys 0.2.0".into(),
        ))
    }

    fn loft(
        &self,
        profiles: &[(Wire2D, Vec3, Vec3)],
        create_solid: bool,
        _ruled: bool,
    ) -> CadResult<Solid> {
        if profiles.len() < 2 {
            return Err(CadError::InvalidProfile(
                "Loft requires at least 2 profiles".into(),
            ));
        }

        // Create loft maker (ruled parameter not available in this version)
        let mut loft = ffi::BRepOffsetAPI_ThruSections_ctor(create_solid);

        // Add all profiles
        for (profile, origin, normal) in profiles {
            if profile.points.len() < 3 {
                return Err(CadError::InvalidProfile(
                    "Each profile must have at least 3 points".into(),
                ));
            }

            // Calculate a perpendicular x_axis for the plane
            let x_axis = if normal.z.abs() < 0.9 {
                normal.cross(Vec3::Z).normalize()
            } else {
                normal.cross(Vec3::X).normalize()
            };
            let wire = self.create_wire(profile, *origin, *normal, x_axis)?;
            loft.pin_mut().AddWire(&wire);
        }

        // Build
        let progress = ffi::Message_ProgressRange_ctor();
        loft.pin_mut().Build(&progress);

        let result = ffi::TopoDS_Shape_to_owned(loft.pin_mut().Shape());
        Ok(self.store_solid(result))
    }

    fn import_step(
        &self,
        path: &std::path::Path,
        options: &StepImportOptions,
    ) -> CadResult<StepImportResult> {
        let path_str = path.to_string_lossy().to_string();

        // Create STEP reader
        let mut reader = ffi::STEPControl_Reader_ctor();

        // Read the file
        let status = ffi::read_step(reader.pin_mut(), path_str);
        if status != ffi::IFSelect_ReturnStatus::IFSelect_RetDone {
            return Err(CadError::StepImport(format!(
                "Failed to read STEP file: {:?}",
                status
            )));
        }

        // Transfer roots to shapes
        let progress = ffi::Message_ProgressRange_ctor();
        let num_roots = reader.pin_mut().TransferRoots(&progress);
        if num_roots == 0 {
            return Err(CadError::StepImport(
                "No valid shapes found in STEP file".into(),
            ));
        }

        // Get the combined shape
        let compound_shape = ffi::one_shape(&reader);

        let mut solids = Vec::new();
        let mut meshes = Vec::new();
        let mut names = Vec::new();

        // Enumerate all solids in the compound
        let mut explorer =
            ffi::TopExp_Explorer_ctor(&compound_shape, ffi::TopAbs_ShapeEnum::TopAbs_SOLID);

        while explorer.More() {
            let solid_shape = explorer.Current();

            // Clone the shape for storage
            let cloned = ffi::TopoDS_Shape_to_owned(solid_shape);

            if options.import_as_solids {
                let solid = self.store_solid(cloned);
                solids.push(solid);
            } else {
                // Tessellate immediately
                let tolerance = options.tessellation_tolerance.unwrap_or(0.1);
                let mesh =
                    self.tessellate_shape(&ffi::TopoDS_Shape_to_owned(solid_shape), tolerance)?;
                meshes.push(mesh);
            }

            names.push(None); // TODO: Extract names from STEP entities

            explorer.pin_mut().Next();
        }

        // If no solids found, try the compound shape directly
        if solids.is_empty() && meshes.is_empty() {
            if options.import_as_solids {
                let solid = self.store_solid(ffi::TopoDS_Shape_to_owned(&compound_shape));
                solids.push(solid);
            } else {
                let tolerance = options.tessellation_tolerance.unwrap_or(0.1);
                let mesh = self.tessellate_shape(&compound_shape, tolerance)?;
                meshes.push(mesh);
            }
            names.push(None);
        }

        Ok(StepImportResult {
            solids,
            meshes,
            names,
        })
    }

    fn export_step(
        &self,
        solid: &Solid,
        path: &std::path::Path,
        _options: &StepExportOptions,
    ) -> CadResult<()> {
        let occ_solid = self
            .get_solid(solid.id)
            .ok_or_else(|| CadError::OperationFailed("Solid not found".into()))?;

        let path_str = path.to_string_lossy().to_string();

        // Create STEP writer
        let mut writer = ffi::STEPControl_Writer_ctor();

        // Transfer shape
        let status = ffi::transfer_shape(writer.pin_mut(), &occ_solid.shape);
        if status != ffi::IFSelect_ReturnStatus::IFSelect_RetDone {
            return Err(CadError::StepExport(format!(
                "Failed to transfer shape: {:?}",
                status
            )));
        }

        // Write file
        let status = ffi::write_step(writer.pin_mut(), path_str);
        if status != ffi::IFSelect_ReturnStatus::IFSelect_RetDone {
            return Err(CadError::StepExport(format!(
                "Failed to write STEP file: {:?}",
                status
            )));
        }

        Ok(())
    }

    fn export_step_multi(
        &self,
        solids: &[&Solid],
        path: &std::path::Path,
        _options: &StepExportOptions,
    ) -> CadResult<()> {
        if solids.is_empty() {
            return Err(CadError::StepExport("No solids to export".into()));
        }

        let path_str = path.to_string_lossy().to_string();

        // Create STEP writer
        let mut writer = ffi::STEPControl_Writer_ctor();

        // Transfer each solid
        for solid in solids {
            let occ_solid = self
                .get_solid(solid.id)
                .ok_or_else(|| CadError::OperationFailed("Solid not found".into()))?;

            let status = ffi::transfer_shape(writer.pin_mut(), &occ_solid.shape);
            if status != ffi::IFSelect_ReturnStatus::IFSelect_RetDone {
                return Err(CadError::StepExport(format!(
                    "Failed to transfer shape: {:?}",
                    status
                )));
            }
        }

        // Write file
        let status = ffi::write_step(writer.pin_mut(), path_str);
        if status != ffi::IFSelect_ReturnStatus::IFSelect_RetDone {
            return Err(CadError::StepExport(format!(
                "Failed to write STEP file: {:?}",
                status
            )));
        }

        Ok(())
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

    /// Tessellate a shape directly (for STEP import)
    fn tessellate_shape(
        &self,
        shape: &cxx::UniquePtr<ffi::TopoDS_Shape>,
        tolerance: f32,
    ) -> CadResult<TessellatedMesh> {
        // Create mesh
        let _mesh_builder = ffi::BRepMesh_IncrementalMesh_ctor(shape, tolerance as f64);

        let mut result = TessellatedMesh::new();

        // Extract triangulation from each face
        let mut explorer = ffi::TopExp_Explorer_ctor(shape, ffi::TopAbs_ShapeEnum::TopAbs_FACE);

        while explorer.More() {
            let face_shape = explorer.Current();
            let face = ffi::TopoDS_cast_to_face(face_shape);

            let mut location = ffi::TopLoc_Location_ctor();
            let triangulation = ffi::BRep_Tool_Triangulation(face, location.pin_mut());

            if !triangulation.IsNull() {
                let tri = ffi::Handle_Poly_Triangulation_Get(&triangulation)
                    .map_err(|e| CadError::TessellationFailed(e.to_string()))?;

                let nb_nodes = tri.NbNodes();
                let nb_triangles = tri.NbTriangles();

                let vertex_offset = result.vertices.len() as u32;

                // Get the transformation from location
                let transform = ffi::TopLoc_Location_Transformation(&location);

                // Extract vertices
                for i in 1..=nb_nodes {
                    let mut node = ffi::Poly_Triangulation_Node(tri, i);
                    node.pin_mut().Transform(&transform);
                    result
                        .vertices
                        .push([node.X() as f32, node.Y() as f32, node.Z() as f32]);
                    // Placeholder normals
                    result.normals.push([0.0, 1.0, 0.0]);
                }

                // Extract triangles
                for i in 1..=nb_triangles {
                    let triangle = tri.Triangle(i);
                    let (n1, n2, n3) = (
                        triangle.Value(1) as u32 - 1 + vertex_offset,
                        triangle.Value(2) as u32 - 1 + vertex_offset,
                        triangle.Value(3) as u32 - 1 + vertex_offset,
                    );

                    // Check face orientation
                    let orientation = face_shape.Orientation();
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

            explorer.pin_mut().Next();
        }

        // Compute normals from triangles
        self.compute_normals(&mut result);

        Ok(result)
    }
}
