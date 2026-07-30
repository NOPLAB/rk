//! OpenCASCADE CAD Kernel Backend
//!
//! Provides bindings to the OpenCASCADE geometry kernel via opencascade-sys.

use glam::Vec3;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

use super::{
    Axis3D, BooleanType, CadError, CadKernel, CadResult, EdgeId, EdgeInfo, FaceId, FaceInfo,
    Region2D, Solid, StepExportOptions, StepImportOptions, StepImportResult, TessellatedMesh,
    Wire2D,
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
            if !edge_maker.IsDone() {
                return Err(CadError::InvalidProfile(
                    "Profile has a segment of no length".into(),
                ));
            }
            let edge = ffi::TopoDS_Edge_to_owned(edge_maker.pin_mut().Edge());
            wire_builder.pin_mut().add_edge(&edge);
        }

        if !wire_builder.IsDone() {
            return Err(CadError::InvalidProfile(
                "Profile does not close into a wire".into(),
            ));
        }

        Ok(ffi::TopoDS_Wire_to_owned(wire_builder.pin_mut().Wire()))
    }

    /// Build the planar face a sweep starts from, from one boundary
    ///
    /// `IsDone` is checked before `Face` because the getter throws on a maker
    /// that failed, and a C++ exception across the cxx bridge aborts the
    /// process rather than unwinding into a `Result`.
    fn create_face(
        &self,
        profile: &Wire2D,
        plane_origin: Vec3,
        plane_x_axis: Vec3,
        plane_y_axis: Vec3,
    ) -> CadResult<cxx::UniquePtr<ffi::TopoDS_Face>> {
        if profile.points.len() < 3 {
            return Err(CadError::InvalidProfile(
                "Profile must have at least 3 points".into(),
            ));
        }

        let wire = self.create_wire(profile, plane_origin, plane_x_axis, plane_y_axis)?;
        let face_maker = ffi::BRepBuilderAPI_MakeFace_wire(&wire, true);
        if !face_maker.IsDone() {
            return Err(CadError::InvalidProfile(
                "Profile does not bound a planar face".into(),
            ));
        }

        Ok(ffi::TopoDS_Face_to_owned(face_maker.Face()))
    }

    /// Sweep a face along a vector
    fn prism(
        &self,
        face: &ffi::TopoDS_Face,
        vec: &ffi::gp_Vec,
    ) -> CadResult<cxx::UniquePtr<ffi::TopoDS_Shape>> {
        let mut prism =
            ffi::BRepPrimAPI_MakePrism_ctor(ffi::cast_face_to_shape(face), vec, false, true);
        if !prism.IsDone() {
            return Err(CadError::OperationFailed("Extrusion failed".into()));
        }
        Ok(ffi::TopoDS_Shape_to_owned(prism.pin_mut().Shape()))
    }

    /// A whole turn is a whole turn
    ///
    /// `angle` arrives as an `f32`, which cannot hold 2π: the nearest float
    /// above it, and the one anything asking for a full revolution will send,
    /// is 1.7e-7 rad *past* the whole turn. OpenCASCADE takes that at face
    /// value and sweeps a solid that laps itself by a sliver — which meshes to
    /// a body of no volume at all, silently. Its own `Precision::Angular` is
    /// 1e-12, far too fine to catch it.
    fn full_turns_stay_full(angle: f32) -> f64 {
        let full = std::f64::consts::TAU;
        let angle = angle as f64;
        if (angle.abs() - full).abs() < 1e-4 {
            full.copysign(angle)
        } else {
            angle.clamp(-full, full)
        }
    }

    /// Sweep a face around an axis
    fn revol(
        &self,
        face: &ffi::TopoDS_Face,
        axis: &ffi::gp_Ax1,
        angle: f32,
    ) -> CadResult<cxx::UniquePtr<ffi::TopoDS_Shape>> {
        let mut revol = ffi::BRepPrimAPI_MakeRevol_ctor(
            ffi::cast_face_to_shape(face),
            axis,
            Self::full_turns_stay_full(angle),
            true,
        );
        if !revol.IsDone() {
            return Err(CadError::OperationFailed("Revolution failed".into()));
        }
        Ok(ffi::TopoDS_Shape_to_owned(revol.pin_mut().Shape()))
    }

    /// Take `tool` out of `base`
    fn cut(
        &self,
        base: &ffi::TopoDS_Shape,
        tool: &ffi::TopoDS_Shape,
    ) -> CadResult<cxx::UniquePtr<ffi::TopoDS_Shape>> {
        let mut cut = ffi::BRepAlgoAPI_Cut_ctor(base, tool);
        if !cut.IsDone() {
            return Err(CadError::BooleanFailed(
                "Cutting an island out of the swept profile failed".into(),
            ));
        }
        Ok(ffi::TopoDS_Shape_to_owned(cut.pin_mut().Shape()))
    }

    /// A key that tells one edge of a solid from another
    ///
    /// `TopExp_Explorer` hands back every edge once per face it belongs to, so
    /// a box arrives as 24 edges rather than 12, and `TopoDS_Shape::IsEqual`
    /// cannot collapse the pairs — the two visits differ in orientation.
    /// Position can: the same edge yields the same curve object both times, so
    /// the numbers come back bit-identical. Start, end and the point halfway
    /// along together separate even two arcs drawn between the same vertices.
    ///
    /// `None` for an edge with no curve, which is skipped rather than numbered.
    fn edge_key(edge: &ffi::TopoDS_Edge) -> Option<[i64; 9]> {
        let mut first = 0.0f64;
        let mut last = 0.0f64;
        let curve = ffi::BRep_Tool_Curve(edge, &mut first, &mut last);
        if curve.IsNull() {
            return None;
        }
        let at = |t: f64| {
            let p = ffi::HandleGeomCurve_Value(&curve, t);
            [
                (p.X() * 1e6).round() as i64,
                (p.Y() * 1e6).round() as i64,
                (p.Z() * 1e6).round() as i64,
            ]
        };
        let (s, e, m) = (at(first), at(last), at((first + last) / 2.0));
        Some([s[0], s[1], s[2], e[0], e[1], e[2], m[0], m[1], m[2]])
    }

    fn axis_of(axis: &Axis3D) -> cxx::UniquePtr<ffi::gp_Ax1> {
        let origin = ffi::new_point(
            axis.origin.x as f64,
            axis.origin.y as f64,
            axis.origin.z as f64,
        );
        let dir = ffi::gp_Dir_ctor(
            axis.direction.x as f64,
            axis.direction.y as f64,
            axis.direction.z as f64,
        );
        ffi::gp_Ax1_ctor(&origin, &dir)
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
        self.extrude_region(
            &Region2D::solid(profile.clone()),
            plane_origin,
            plane_x_axis,
            plane_y_axis,
            direction,
            distance,
        )
    }

    /// Sweep the outer boundary, then take the same sweep of each island back out
    ///
    /// The bound `BRepBuilderAPI_MakeFace` only accepts a single wire, so a
    /// face cannot be handed its islands the way truck's `try_attach_plane`
    /// takes them. Cutting them out afterwards costs one boolean per island
    /// and reaches the same solid — and it does not care which way round the
    /// island is wound, which the face route very much would.
    fn extrude_region(
        &self,
        region: &Region2D,
        plane_origin: Vec3,
        plane_x_axis: Vec3,
        plane_y_axis: Vec3,
        direction: Vec3,
        distance: f32,
    ) -> CadResult<Solid> {
        let sweep = direction * distance;
        if sweep.length() < f32::EPSILON {
            return Err(CadError::InvalidProfile("Extrusion has no length".into()));
        }
        let vec = |v: Vec3| ffi::new_vec(v.x as f64, v.y as f64, v.z as f64);

        let outer = self.create_face(&region.outer, plane_origin, plane_x_axis, plane_y_axis)?;
        let mut shape = self.prism(&outer, &vec(sweep))?;

        // The cutting tool is made to stick out past both ends. Left flush it
        // would ask the boolean to resolve a face lying exactly on a face,
        // which is the one case it is slowest and least sure about — and the
        // overshoot cannot reach material, since there is none out there.
        let overshoot = sweep.normalize() * (sweep.length() * 0.01).max(1e-4);
        for hole in &region.holes {
            let face =
                self.create_face(hole, plane_origin - overshoot, plane_x_axis, plane_y_axis)?;
            let tool = self.prism(&face, &vec(sweep + overshoot * 2.0))?;
            shape = self.cut(&shape, &tool)?;
        }

        Ok(self.store_solid(shape))
    }

    fn supports_holes(&self) -> bool {
        true
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
        self.revolve_region(
            &Region2D::solid(profile.clone()),
            plane_origin,
            plane_x_axis,
            plane_y_axis,
            axis,
            angle,
        )
    }

    /// As [`Self::extrude_region`], with the islands revolved rather than swept
    ///
    /// Nothing is overshot here: a partial revolution leaves the tool's end
    /// caps sitting in the body's own end caps, which is the ordinary
    /// same-domain case a boolean is built for, and a full turn has no caps
    /// at all. Over-rotating would be the harmful choice — past a full turn
    /// the sweep folds back through itself.
    fn revolve_region(
        &self,
        region: &Region2D,
        plane_origin: Vec3,
        plane_x_axis: Vec3,
        plane_y_axis: Vec3,
        axis: &Axis3D,
        angle: f32,
    ) -> CadResult<Solid> {
        // Rejected here rather than by an `IsDone` further down: a revolution
        // of no angle raises out of `BRepSweep_Rotation`'s own constructor,
        // and a C++ throw crossing the cxx bridge takes the process with it
        if !angle.is_finite() || (angle as f64).abs() <= 1e-9 {
            return Err(CadError::InvalidProfile("Revolution has no angle".into()));
        }

        let gp_axis = Self::axis_of(axis);

        let outer = self.create_face(&region.outer, plane_origin, plane_x_axis, plane_y_axis)?;
        let mut shape = self.revol(&outer, &gp_axis, angle)?;

        for hole in &region.holes {
            let face = self.create_face(hole, plane_origin, plane_x_axis, plane_y_axis)?;
            let tool = self.revol(&face, &gp_axis, angle)?;
            shape = self.cut(&shape, &tool)?;
        }

        Ok(self.store_solid(shape))
    }

    fn boolean(&self, a: &Solid, b: &Solid, op: BooleanType) -> CadResult<Solid> {
        let solid_a = self
            .get_solid(a.id)
            .ok_or_else(|| CadError::OperationFailed("First solid not found".into()))?;

        let solid_b = self
            .get_solid(b.id)
            .ok_or_else(|| CadError::OperationFailed("Second solid not found".into()))?;

        // A boolean that fails hands back a null shape rather than saying so,
        // and a null shape tessellates to nothing at all — an operation that
        // did not work must not look like one that produced an empty body.
        let failed = |op: &str| CadError::BooleanFailed(format!("{op} failed"));

        let result_shape = match op {
            BooleanType::Union => {
                let mut fuse = ffi::BRepAlgoAPI_Fuse_ctor(&solid_a.shape, &solid_b.shape);
                if !fuse.IsDone() {
                    return Err(failed("Union"));
                }
                ffi::TopoDS_Shape_to_owned(fuse.pin_mut().Shape())
            }
            BooleanType::Subtract => {
                let mut cut = ffi::BRepAlgoAPI_Cut_ctor(&solid_a.shape, &solid_b.shape);
                if !cut.IsDone() {
                    return Err(failed("Subtraction"));
                }
                ffi::TopoDS_Shape_to_owned(cut.pin_mut().Shape())
            }
            BooleanType::Intersect => {
                let mut common = ffi::BRepAlgoAPI_Common_ctor(&solid_a.shape, &solid_b.shape);
                if !common.IsDone() {
                    return Err(failed("Intersection"));
                }
                ffi::TopoDS_Shape_to_owned(common.pin_mut().Shape())
            }
        };

        Ok(self.store_solid(result_shape))
    }

    fn tessellate(&self, solid: &Solid, tolerance: f32) -> CadResult<TessellatedMesh> {
        // The lock is held across the meshing, not just the lookup. Copying a
        // `TopoDS_Shape` shares the topology underneath rather than duplicating
        // it, and meshing writes the triangulation back into that shared
        // topology — so two threads meshing bodies that came out of the same
        // boolean would be writing the same memory. rk-mcp takes screenshots
        // on its own schedule beside the app, so it is reachable.
        let solids = self.solids.lock().unwrap();
        let stored = solids
            .get(&solid.id)
            .ok_or_else(|| CadError::TessellationFailed("Solid not found".into()))?;

        Self::mesh_of(&stored.shape, tolerance)
    }

    fn create_box(&self, center: Vec3, size: Vec3) -> CadResult<Solid> {
        let half = size * 0.5;
        let min = center - half;

        let p1 = ffi::new_point(min.x as f64, min.y as f64, min.z as f64);

        // Unlike the sweeps, the primitive makers do not build in their
        // constructor, and `IsDone` before `Build` is false for a perfectly
        // good box
        let mut box_maker =
            ffi::BRepPrimAPI_MakeBox_ctor(&p1, size.x as f64, size.y as f64, size.z as f64);
        box_maker
            .pin_mut()
            .Build(&ffi::Message_ProgressRange_ctor());
        if !box_maker.IsDone() {
            return Err(CadError::OperationFailed("Box has no size".into()));
        }
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
        cylinder.pin_mut().Build(&ffi::Message_ProgressRange_ctor());
        if !cylinder.IsDone() {
            return Err(CadError::OperationFailed(
                "Cylinder has no radius or no height".into(),
            ));
        }
        Ok(self.store_solid(ffi::TopoDS_Shape_to_owned(cylinder.pin_mut().Shape())))
    }

    fn create_sphere(&self, center: Vec3, radius: f32) -> CadResult<Solid> {
        // Create sphere centered at the specified point
        let origin = ffi::new_point(center.x as f64, center.y as f64, center.z as f64);
        let dir = ffi::gp_Dir_ctor(0.0, 0.0, 1.0);
        let ax2 = ffi::gp_Ax2_ctor(&origin, &dir);

        // The angle is how far the sphere sweeps round its axis, so a whole
        // one is a full turn. Half a turn — which reads like "PI for a full
        // sphere" — builds a spherical wedge: a hemisphere.
        let mut sphere =
            ffi::BRepPrimAPI_MakeSphere_ctor(&ax2, radius as f64, std::f64::consts::TAU);
        sphere.pin_mut().Build(&ffi::Message_ProgressRange_ctor());
        if !sphere.IsDone() {
            return Err(CadError::OperationFailed("Sphere has no radius".into()));
        }
        Ok(self.store_solid(ffi::TopoDS_Shape_to_owned(sphere.pin_mut().Shape())))
    }

    fn get_edges(&self, solid: &Solid) -> CadResult<Vec<EdgeInfo>> {
        let occ_solid = self
            .get_solid(solid.id)
            .ok_or_else(|| CadError::OperationFailed("Solid not found".into()))?;

        let mut edges = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Use TopExp_Explorer to iterate through all edges
        let mut explorer =
            ffi::TopExp_Explorer_ctor(&occ_solid.shape, ffi::TopAbs_ShapeEnum::TopAbs_EDGE);

        while explorer.More() {
            let edge = ffi::TopoDS_cast_to_edge(explorer.Current());

            // Every edge is met once per face it borders, and `EdgeId.index`
            // is what a fillet later selects by — so it counts unique edges,
            // in the order they are first met. `fillet` and `chamfer` walk the
            // same explorer under the same rule to stay in step.
            if let Some(key) = Self::edge_key(edge)
                && seen.insert(key)
            {
                let mut first = 0.0f64;
                let mut last = 0.0f64;
                let curve = ffi::BRep_Tool_Curve(edge, &mut first, &mut last);

                let at = |t: f64| {
                    let p = ffi::HandleGeomCurve_Value(&curve, t);
                    Vec3::new(p.X() as f32, p.Y() as f32, p.Z() as f32)
                };
                // Length along the curve, not across it — the chord of a
                // closed circle is zero, and it is exactly the round edges a
                // fillet is reached for
                let mut props = ffi::GProp_GProps_ctor();
                ffi::BRepGProp_LinearProperties(explorer.Current(), props.pin_mut());

                edges.push(EdgeInfo {
                    id: EdgeId::new(solid.id, edges.len() as u32),
                    start: at(first),
                    end: at(last),
                    midpoint: at((first + last) / 2.0),
                    length: props.Mass() as f32,
                });
            }

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
                // Area and centre both come out of the surface properties, so
                // the centre is the face's own centroid rather than wherever
                // some parameter pair happens to land
                let mut props = ffi::GProp_GProps_ctor();
                ffi::BRepGProp_SurfaceProperties(face_shape, props.pin_mut());
                let area = props.Mass() as f32;
                let centre = ffi::GProp_GProps_CentreOfMass(&props);
                let center = Vec3::new(centre.X() as f32, centre.Y() as f32, centre.Z() as f32);

                // The centroid's (u, v) is not something the bindings can give
                // back, so the normal is still read at an arbitrary parameter.
                // Exact for a plane, where the normal is the same everywhere;
                // for a curved face it is a normal of the surface, not of the
                // centre. A reversed face points into the solid, so flip it.
                let brep_face = ffi::BRepGProp_Face_ctor(face);
                let mut at = ffi::new_point(0.0, 0.0, 0.0);
                let mut normal_vec = ffi::new_vec(0.0, 0.0, 1.0);
                brep_face.Normal(0.5, 0.5, at.pin_mut(), normal_vec.pin_mut());

                let mut normal = Vec3::new(
                    normal_vec.X() as f32,
                    normal_vec.Y() as f32,
                    normal_vec.Z() as f32,
                );
                if face_shape.Orientation() == ffi::TopAbs_Orientation::TopAbs_REVERSED {
                    normal = -normal;
                }

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

        // Number the edges exactly as `get_edges` did, or the caller rounds
        // an edge it never picked
        let mut edge_index = 0u32;
        let mut seen = std::collections::HashSet::new();
        let mut explorer =
            ffi::TopExp_Explorer_ctor(&occ_solid.shape, ffi::TopAbs_ShapeEnum::TopAbs_EDGE);

        while explorer.More() {
            let edge = ffi::TopoDS_cast_to_edge(explorer.Current());
            if let Some(key) = Self::edge_key(edge)
                && seen.insert(key)
            {
                if edges.iter().any(|e| e.index == edge_index) {
                    fillet.pin_mut().add_edge(radius as f64, edge);
                }
                edge_index += 1;
            }

            explorer.pin_mut().Next();
        }

        // Build the result
        let progress = ffi::Message_ProgressRange_ctor();
        fillet.pin_mut().Build(&progress);
        if !fillet.IsDone() {
            return Err(CadError::OperationFailed(
                "Fillet failed — the radius is probably larger than the edge allows".into(),
            ));
        }

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

        // Same numbering as `get_edges` and `fillet`
        let mut edge_index = 0u32;
        let mut seen = std::collections::HashSet::new();
        let mut explorer =
            ffi::TopExp_Explorer_ctor(&occ_solid.shape, ffi::TopAbs_ShapeEnum::TopAbs_EDGE);

        while explorer.More() {
            let edge = ffi::TopoDS_cast_to_edge(explorer.Current());
            if let Some(key) = Self::edge_key(edge)
                && seen.insert(key)
            {
                if edges.iter().any(|e| e.index == edge_index) {
                    chamfer.pin_mut().add_edge(distance as f64, edge);
                }
                edge_index += 1;
            }

            explorer.pin_mut().Next();
        }

        // Build the result
        let progress = ffi::Message_ProgressRange_ctor();
        chamfer.pin_mut().Build(&progress);
        if !chamfer.IsDone() {
            return Err(CadError::OperationFailed(
                "Chamfer failed — the distance is probably larger than the edge allows".into(),
            ));
        }

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
        if !thick_solid.IsDone() {
            return Err(CadError::OperationFailed(
                "Shell failed — the wall is probably thicker than the body".into(),
            ));
        }

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

            // Two axes lying *in* the plane. Handing the normal over as the
            // X axis stands every section on its edge, in a plane at right
            // angles to the one asked for.
            let x_axis = if normal.z.abs() < 0.9 {
                normal.cross(Vec3::Z).normalize()
            } else {
                normal.cross(Vec3::X).normalize()
            };
            let y_axis = normal.cross(x_axis).normalize();
            let wire = self.create_wire(profile, *origin, x_axis, y_axis)?;
            loft.pin_mut().AddWire(&wire);
        }

        // Build
        let progress = ffi::Message_ProgressRange_ctor();
        loft.pin_mut().Build(&progress);
        if !loft.IsDone() {
            return Err(CadError::OperationFailed(
                "Loft failed — the sections may not be compatible".into(),
            ));
        }

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
        let compound_shape = ffi::one_shape_step(&reader);

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
                let mesh = Self::mesh_of(solid_shape, tolerance)?;
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
                let mesh = Self::mesh_of(&compound_shape, tolerance)?;
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

    /// Write a solid out as STEP
    ///
    /// Two things to know before this is wired to a command. It prints its
    /// transfer statistics **to stdout**, which is rk-mcp's JSON-RPC channel —
    /// an agent asking for a STEP export would corrupt the protocol. And it
    /// declares millimetres while writing the model's own numbers, which are
    /// metres, so what it writes is a thousand times smaller than it says.
    /// Import compensates for the same convention on the way in; export does
    /// not yet.
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
    /// Normals accumulated from the triangle soup
    ///
    /// The fallback for a face whose triangulation carries none of its own —
    /// a chord-averaged normal shades a curved surface visibly worse than the
    /// surface's own.
    fn derive_normals(mesh: &mut TessellatedMesh) {
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

    /// Triangles for a shape, with OpenCASCADE's own surface normals
    fn mesh_of(shape: &ffi::TopoDS_Shape, tolerance: f32) -> CadResult<TessellatedMesh> {
        // The deflection BRepMesh wants is an absolute length, and the one
        // constant the app hands it is a tenth of a metre — coarser than most
        // parts are big. Bounding it by the body's own size keeps a small
        // part's curves round without making a large one needlessly heavy.
        let mut bounds = ffi::Bnd_Box_ctor();
        ffi::BRepBndLib_Add(shape, bounds.pin_mut(), false);
        if bounds.IsVoid() {
            return Err(CadError::TessellationFailed(
                "The operation left nothing to mesh".into(),
            ));
        }
        let (min, max) = (
            ffi::Bnd_Box_CornerMin(&bounds),
            ffi::Bnd_Box_CornerMax(&bounds),
        );
        let diagonal = ((max.X() - min.X()).powi(2)
            + (max.Y() - min.Y()).powi(2)
            + (max.Z() - min.Z()).powi(2))
        .sqrt();
        let deflection = (tolerance as f64)
            .min(diagonal * 0.005)
            .max(diagonal * 1e-4);

        let mesh_builder = ffi::BRepMesh_IncrementalMesh_ctor(shape, deflection);
        if !mesh_builder.IsDone() {
            return Err(CadError::TessellationFailed("Meshing failed".into()));
        }

        let mut result = TessellatedMesh::new();
        let mut every_face_had_normals = true;

        let mut explorer = ffi::TopExp_Explorer_ctor(shape, ffi::TopAbs_ShapeEnum::TopAbs_FACE);

        while explorer.More() {
            let face_shape = explorer.Current();
            let face = ffi::TopoDS_cast_to_face(face_shape);
            // A reversed face's outward side is the other one, which flips
            // both its normals and the winding of its triangles
            let reversed = face_shape.Orientation() == ffi::TopAbs_Orientation::TopAbs_REVERSED;

            let mut location = ffi::TopLoc_Location_ctor();
            let triangulation = ffi::BRep_Tool_Triangulation(face, location.pin_mut());

            if !triangulation.IsNull() {
                // Fills the triangulation's normals in from the surface, so a
                // cylinder shades as a cylinder rather than as its own facets
                ffi::compute_normals(face, &triangulation);

                let tri = ffi::HandlePoly_Triangulation_Get(&triangulation)
                    .map_err(|e: cxx::Exception| CadError::TessellationFailed(e.to_string()))?;

                let vertex_offset = result.vertices.len() as u32;
                let transform = ffi::TopLoc_Location_Transformation(&location);
                let has_normals = tri.HasNormals();
                every_face_had_normals &= has_normals;

                for i in 1..=tri.NbNodes() {
                    let mut node = ffi::Poly_Triangulation_Node(tri, i);
                    node.pin_mut().Transform(&transform);
                    result
                        .vertices
                        .push([node.X() as f32, node.Y() as f32, node.Z() as f32]);

                    result.normals.push(if has_normals {
                        let mut n = ffi::Poly_Triangulation_Normal(tri, i);
                        n.pin_mut().Transform(&transform);
                        let sign = if reversed { -1.0 } else { 1.0 };
                        [
                            (n.X() * sign) as f32,
                            (n.Y() * sign) as f32,
                            (n.Z() * sign) as f32,
                        ]
                    } else {
                        [0.0, 0.0, 0.0]
                    });
                }

                for i in 1..=tri.NbTriangles() {
                    let triangle = tri.Triangle(i);
                    let (n1, n2, n3) = (
                        triangle.Value(1) as u32 - 1 + vertex_offset,
                        triangle.Value(2) as u32 - 1 + vertex_offset,
                        triangle.Value(3) as u32 - 1 + vertex_offset,
                    );

                    if reversed {
                        result.indices.extend([n1, n3, n2]);
                    } else {
                        result.indices.extend([n1, n2, n3]);
                    }
                }
            }

            explorer.pin_mut().Next();
        }

        // A mesh with nothing in it is what a failed operation looks like from
        // here, and handing one back as a success hides the failure completely
        if result.vertices.is_empty() {
            return Err(CadError::TessellationFailed(
                "The body has no surface to mesh".into(),
            ));
        }

        if !every_face_had_normals {
            Self::derive_normals(&mut result);
        }

        Ok(result)
    }
}
