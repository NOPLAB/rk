//! Feature Operations
//!
//! Provides parametric feature operations like extrude, revolve, and boolean
//! that operate on sketches to create 3D solids.

use glam::Vec3;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::kernel::{Axis3D, BooleanType, CadKernel, EdgeId, FaceId, Solid, TessellatedMesh};
use crate::sketch::Sketch;

/// Feature-related errors
#[derive(Debug, Clone, Error)]
pub enum FeatureError {
    #[error("Sketch error: {0}")]
    SketchError(#[from] crate::sketch::SketchError),

    #[error("CAD kernel error: {0}")]
    CadError(#[from] crate::kernel::CadError),

    #[error("Invalid feature: {0}")]
    InvalidFeature(String),

    #[error("Feature not found: {0}")]
    FeatureNotFound(Uuid),

    #[error("Rebuild failed: {0}")]
    RebuildFailed(String),
}

/// Result type for feature operations
pub type FeatureResult<T> = Result<T, FeatureError>;

/// Direction for extrusion
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum ExtrudeDirection {
    /// Extrude in the positive normal direction
    #[default]
    Positive,
    /// Extrude in the negative normal direction
    Negative,
    /// Extrude symmetrically in both directions
    Symmetric,
}

/// Boolean operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BooleanOp {
    /// Create new body
    #[default]
    New,
    /// Add to existing body
    Join,
    /// Remove from existing body
    Cut,
    /// Keep only intersection
    Intersect,
}

impl From<BooleanOp> for Option<BooleanType> {
    fn from(op: BooleanOp) -> Self {
        match op {
            BooleanOp::New => None,
            BooleanOp::Join => Some(BooleanType::Union),
            BooleanOp::Cut => Some(BooleanType::Subtract),
            BooleanOp::Intersect => Some(BooleanType::Intersect),
        }
    }
}

/// A parametric feature that modifies geometry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Feature {
    /// Extrude a sketch profile
    Extrude {
        /// Unique identifier
        id: Uuid,
        /// Name of the feature
        name: String,
        /// Reference to the sketch
        sketch_id: Uuid,
        /// Extrusion distance
        distance: f32,
        /// Extrusion direction
        direction: ExtrudeDirection,
        /// Boolean operation with existing body
        boolean_op: BooleanOp,
        /// Target body ID (for boolean operations)
        target_body: Option<Uuid>,
        /// Draft angle in radians (0 = no draft)
        draft_angle: f32,
        /// Whether the feature is suppressed
        #[serde(default)]
        suppressed: bool,
    },

    /// Revolve a sketch profile around an axis
    Revolve {
        /// Unique identifier
        id: Uuid,
        /// Name of the feature
        name: String,
        /// Reference to the sketch
        sketch_id: Uuid,
        /// Axis origin
        axis_origin: Vec3,
        /// Axis direction
        axis_direction: Vec3,
        /// Rotation angle in radians
        angle: f32,
        /// Boolean operation with existing body
        boolean_op: BooleanOp,
        /// Target body ID (for boolean operations)
        target_body: Option<Uuid>,
        /// Whether the feature is suppressed
        #[serde(default)]
        suppressed: bool,
    },

    /// Boolean operation between two bodies
    Boolean {
        /// Unique identifier
        id: Uuid,
        /// Name of the feature
        name: String,
        /// Target body
        target_body: Uuid,
        /// Tool body
        tool_body: Uuid,
        /// Operation type
        operation: BooleanOp,
        /// Whether the feature is suppressed
        #[serde(default)]
        suppressed: bool,
    },

    /// Fillet edges
    Fillet {
        /// Unique identifier
        id: Uuid,
        /// Name of the feature
        name: String,
        /// Body to modify
        body_id: Uuid,
        /// Fillet radius
        radius: f32,
        /// Edge IDs to fillet
        edges: Vec<EdgeId>,
        /// Whether the feature is suppressed
        #[serde(default)]
        suppressed: bool,
    },

    /// Chamfer edges
    Chamfer {
        /// Unique identifier
        id: Uuid,
        /// Name of the feature
        name: String,
        /// Body to modify
        body_id: Uuid,
        /// Chamfer distance
        distance: f32,
        /// Edge IDs to chamfer
        edges: Vec<EdgeId>,
        /// Whether the feature is suppressed
        #[serde(default)]
        suppressed: bool,
    },

    /// Shell (hollow out a solid)
    Shell {
        /// Unique identifier
        id: Uuid,
        /// Name of the feature
        name: String,
        /// Body to modify
        body_id: Uuid,
        /// Wall thickness
        thickness: f32,
        /// Faces to remove (create openings)
        faces_to_remove: Vec<FaceId>,
        /// Whether the feature is suppressed
        #[serde(default)]
        suppressed: bool,
    },

    /// Sweep a profile along a path
    Sweep {
        /// Unique identifier
        id: Uuid,
        /// Name of the feature
        name: String,
        /// Profile sketch ID
        profile_sketch_id: Uuid,
        /// Path sketch ID
        path_sketch_id: Uuid,
        /// Boolean operation with existing body
        boolean_op: BooleanOp,
        /// Target body ID (for boolean operations)
        target_body: Option<Uuid>,
        /// Whether the feature is suppressed
        #[serde(default)]
        suppressed: bool,
    },

    /// Loft between multiple profiles
    Loft {
        /// Unique identifier
        id: Uuid,
        /// Name of the feature
        name: String,
        /// Profile sketch IDs (in order)
        profile_sketch_ids: Vec<Uuid>,
        /// Whether to create a solid (vs shell)
        create_solid: bool,
        /// Whether to use ruled surfaces
        ruled: bool,
        /// Boolean operation with existing body
        boolean_op: BooleanOp,
        /// Target body ID (for boolean operations)
        target_body: Option<Uuid>,
        /// Whether the feature is suppressed
        #[serde(default)]
        suppressed: bool,
    },
}

impl Feature {
    /// Get the unique ID of this feature
    pub fn id(&self) -> Uuid {
        match self {
            Feature::Extrude { id, .. } => *id,
            Feature::Revolve { id, .. } => *id,
            Feature::Boolean { id, .. } => *id,
            Feature::Fillet { id, .. } => *id,
            Feature::Chamfer { id, .. } => *id,
            Feature::Shell { id, .. } => *id,
            Feature::Sweep { id, .. } => *id,
            Feature::Loft { id, .. } => *id,
        }
    }

    /// Get the name of this feature
    pub fn name(&self) -> &str {
        match self {
            Feature::Extrude { name, .. } => name,
            Feature::Revolve { name, .. } => name,
            Feature::Boolean { name, .. } => name,
            Feature::Fillet { name, .. } => name,
            Feature::Chamfer { name, .. } => name,
            Feature::Shell { name, .. } => name,
            Feature::Sweep { name, .. } => name,
            Feature::Loft { name, .. } => name,
        }
    }

    /// Get the type name of this feature
    pub fn type_name(&self) -> &'static str {
        match self {
            Feature::Extrude { .. } => "Extrude",
            Feature::Revolve { .. } => "Revolve",
            Feature::Boolean { .. } => "Boolean",
            Feature::Fillet { .. } => "Fillet",
            Feature::Chamfer { .. } => "Chamfer",
            Feature::Shell { .. } => "Shell",
            Feature::Sweep { .. } => "Sweep",
            Feature::Loft { .. } => "Loft",
        }
    }

    /// Check if the feature is suppressed
    pub fn is_suppressed(&self) -> bool {
        match self {
            Feature::Extrude { suppressed, .. } => *suppressed,
            Feature::Revolve { suppressed, .. } => *suppressed,
            Feature::Boolean { suppressed, .. } => *suppressed,
            Feature::Fillet { suppressed, .. } => *suppressed,
            Feature::Chamfer { suppressed, .. } => *suppressed,
            Feature::Shell { suppressed, .. } => *suppressed,
            Feature::Sweep { suppressed, .. } => *suppressed,
            Feature::Loft { suppressed, .. } => *suppressed,
        }
    }

    /// Set the suppressed state
    pub fn set_suppressed(&mut self, value: bool) {
        match self {
            Feature::Extrude { suppressed, .. } => *suppressed = value,
            Feature::Revolve { suppressed, .. } => *suppressed = value,
            Feature::Boolean { suppressed, .. } => *suppressed = value,
            Feature::Fillet { suppressed, .. } => *suppressed = value,
            Feature::Chamfer { suppressed, .. } => *suppressed = value,
            Feature::Shell { suppressed, .. } => *suppressed = value,
            Feature::Sweep { suppressed, .. } => *suppressed = value,
            Feature::Loft { suppressed, .. } => *suppressed = value,
        }
    }

    /// Create a new extrude feature
    pub fn extrude(
        name: impl Into<String>,
        sketch_id: Uuid,
        distance: f32,
        direction: ExtrudeDirection,
    ) -> Self {
        Feature::Extrude {
            id: Uuid::new_v4(),
            name: name.into(),
            sketch_id,
            distance,
            direction,
            boolean_op: BooleanOp::New,
            target_body: None,
            draft_angle: 0.0,
            suppressed: false,
        }
    }

    /// Create a new revolve feature
    pub fn revolve(name: impl Into<String>, sketch_id: Uuid, axis: Axis3D, angle: f32) -> Self {
        Feature::Revolve {
            id: Uuid::new_v4(),
            name: name.into(),
            sketch_id,
            axis_origin: axis.origin,
            axis_direction: axis.direction,
            angle,
            boolean_op: BooleanOp::New,
            target_body: None,
            suppressed: false,
        }
    }

    /// Create a new fillet feature
    pub fn fillet(name: impl Into<String>, body_id: Uuid, edges: Vec<EdgeId>, radius: f32) -> Self {
        Feature::Fillet {
            id: Uuid::new_v4(),
            name: name.into(),
            body_id,
            radius,
            edges,
            suppressed: false,
        }
    }

    /// Create a new chamfer feature
    pub fn chamfer(
        name: impl Into<String>,
        body_id: Uuid,
        edges: Vec<EdgeId>,
        distance: f32,
    ) -> Self {
        Feature::Chamfer {
            id: Uuid::new_v4(),
            name: name.into(),
            body_id,
            distance,
            edges,
            suppressed: false,
        }
    }

    /// Create a new shell feature
    pub fn shell(
        name: impl Into<String>,
        body_id: Uuid,
        thickness: f32,
        faces_to_remove: Vec<FaceId>,
    ) -> Self {
        Feature::Shell {
            id: Uuid::new_v4(),
            name: name.into(),
            body_id,
            thickness,
            faces_to_remove,
            suppressed: false,
        }
    }

    /// Create a new sweep feature
    pub fn sweep(name: impl Into<String>, profile_sketch_id: Uuid, path_sketch_id: Uuid) -> Self {
        Feature::Sweep {
            id: Uuid::new_v4(),
            name: name.into(),
            profile_sketch_id,
            path_sketch_id,
            boolean_op: BooleanOp::New,
            target_body: None,
            suppressed: false,
        }
    }

    /// Create a new loft feature
    pub fn loft(
        name: impl Into<String>,
        profile_sketch_ids: Vec<Uuid>,
        create_solid: bool,
        ruled: bool,
    ) -> Self {
        Feature::Loft {
            id: Uuid::new_v4(),
            name: name.into(),
            profile_sketch_ids,
            create_solid,
            ruled,
            boolean_op: BooleanOp::New,
            target_body: None,
            suppressed: false,
        }
    }

    /// Execute this feature to produce a solid
    pub fn execute(
        &self,
        kernel: &dyn CadKernel,
        sketches: &std::collections::HashMap<Uuid, Sketch>,
        existing_bodies: &std::collections::HashMap<Uuid, Solid>,
    ) -> FeatureResult<Solid> {
        if self.is_suppressed() {
            return Err(FeatureError::InvalidFeature("Feature is suppressed".into()));
        }

        match self {
            Feature::Extrude {
                sketch_id,
                distance,
                direction,
                boolean_op,
                target_body,
                ..
            } => {
                let sketch =
                    sketches
                        .get(sketch_id)
                        .ok_or(FeatureError::InvalidFeature(format!(
                            "Sketch {} not found",
                            sketch_id
                        )))?;

                // Extract profiles from sketch
                let profiles = sketch.extract_profiles()?;

                if profiles.is_empty() {
                    return Err(FeatureError::InvalidFeature(
                        "No closed profiles found".into(),
                    ));
                }

                // Calculate extrusion direction and distance
                let (extrude_dir, extrude_dist) = match direction {
                    ExtrudeDirection::Positive => (sketch.plane.normal, *distance),
                    ExtrudeDirection::Negative => (-sketch.plane.normal, *distance),
                    ExtrudeDirection::Symmetric => (sketch.plane.normal, *distance / 2.0),
                };

                // Extrude the first profile (for now)
                let profile = &profiles[0];
                let mut solid = kernel.extrude(
                    profile,
                    sketch.plane.origin,
                    sketch.plane.normal,
                    sketch.plane.x_axis,
                    extrude_dir,
                    extrude_dist,
                )?;

                // For symmetric, extrude in the other direction and union
                if matches!(direction, ExtrudeDirection::Symmetric) {
                    let solid2 = kernel.extrude(
                        profile,
                        sketch.plane.origin,
                        sketch.plane.normal,
                        sketch.plane.x_axis,
                        -extrude_dir,
                        extrude_dist,
                    )?;
                    solid = kernel.boolean(&solid, &solid2, BooleanType::Union)?;
                }

                // Apply boolean operation with target body
                if let (Some(op), Some(target_id)) =
                    (Option::<BooleanType>::from(*boolean_op), target_body)
                    && let Some(target) = existing_bodies.get(target_id)
                {
                    solid = kernel.boolean(target, &solid, op)?;
                }

                Ok(solid)
            }

            Feature::Revolve {
                sketch_id,
                axis_origin,
                axis_direction,
                angle,
                boolean_op,
                target_body,
                ..
            } => {
                let sketch =
                    sketches
                        .get(sketch_id)
                        .ok_or(FeatureError::InvalidFeature(format!(
                            "Sketch {} not found",
                            sketch_id
                        )))?;

                let profiles = sketch.extract_profiles()?;

                if profiles.is_empty() {
                    return Err(FeatureError::InvalidFeature(
                        "No closed profiles found".into(),
                    ));
                }

                let axis = Axis3D::new(*axis_origin, *axis_direction);
                let profile = &profiles[0];

                let mut solid = kernel.revolve(
                    profile,
                    sketch.plane.origin,
                    sketch.plane.normal,
                    sketch.plane.x_axis,
                    &axis,
                    *angle,
                )?;

                // Apply boolean operation
                if let (Some(op), Some(target_id)) =
                    (Option::<BooleanType>::from(*boolean_op), target_body)
                    && let Some(target) = existing_bodies.get(target_id)
                {
                    solid = kernel.boolean(target, &solid, op)?;
                }

                Ok(solid)
            }

            Feature::Boolean {
                target_body,
                tool_body,
                operation,
                ..
            } => {
                let target = existing_bodies
                    .get(target_body)
                    .ok_or(FeatureError::InvalidFeature("Target body not found".into()))?;

                let tool = existing_bodies
                    .get(tool_body)
                    .ok_or(FeatureError::InvalidFeature("Tool body not found".into()))?;

                let op = Option::<BooleanType>::from(*operation).ok_or(
                    FeatureError::InvalidFeature("Invalid boolean operation".into()),
                )?;

                kernel.boolean(target, tool, op).map_err(|e| e.into())
            }

            Feature::Fillet {
                body_id,
                radius,
                edges,
                ..
            } => {
                let body = existing_bodies
                    .get(body_id)
                    .ok_or(FeatureError::InvalidFeature("Body not found".into()))?;

                kernel.fillet(body, edges, *radius).map_err(|e| e.into())
            }

            Feature::Chamfer {
                body_id,
                distance,
                edges,
                ..
            } => {
                let body = existing_bodies
                    .get(body_id)
                    .ok_or(FeatureError::InvalidFeature("Body not found".into()))?;

                kernel.chamfer(body, edges, *distance).map_err(|e| e.into())
            }

            Feature::Shell {
                body_id,
                thickness,
                faces_to_remove,
                ..
            } => {
                let body = existing_bodies
                    .get(body_id)
                    .ok_or(FeatureError::InvalidFeature("Body not found".into()))?;

                kernel
                    .shell(body, *thickness, faces_to_remove)
                    .map_err(|e| e.into())
            }

            Feature::Sweep {
                profile_sketch_id,
                path_sketch_id,
                boolean_op,
                target_body,
                ..
            } => {
                let profile_sketch =
                    sketches
                        .get(profile_sketch_id)
                        .ok_or(FeatureError::InvalidFeature(format!(
                            "Profile sketch {} not found",
                            profile_sketch_id
                        )))?;

                let path_sketch =
                    sketches
                        .get(path_sketch_id)
                        .ok_or(FeatureError::InvalidFeature(format!(
                            "Path sketch {} not found",
                            path_sketch_id
                        )))?;

                // Extract profiles
                let profiles = profile_sketch.extract_profiles()?;
                if profiles.is_empty() {
                    return Err(FeatureError::InvalidFeature(
                        "No closed profiles found in profile sketch".into(),
                    ));
                }

                // Extract path (can be open)
                let path_profiles = path_sketch.extract_profiles()?;
                if path_profiles.is_empty() {
                    return Err(FeatureError::InvalidFeature(
                        "No path found in path sketch".into(),
                    ));
                }

                let mut solid = kernel.sweep(
                    &profiles[0],
                    profile_sketch.plane.origin,
                    profile_sketch.plane.normal,
                    &path_profiles[0],
                    path_sketch.plane.origin,
                    path_sketch.plane.normal,
                )?;

                // Apply boolean operation
                if let (Some(op), Some(target_id)) =
                    (Option::<BooleanType>::from(*boolean_op), target_body)
                    && let Some(target) = existing_bodies.get(target_id)
                {
                    solid = kernel.boolean(target, &solid, op)?;
                }

                Ok(solid)
            }

            Feature::Loft {
                profile_sketch_ids,
                create_solid,
                ruled,
                boolean_op,
                target_body,
                ..
            } => {
                if profile_sketch_ids.len() < 2 {
                    return Err(FeatureError::InvalidFeature(
                        "Loft requires at least 2 profiles".into(),
                    ));
                }

                // Collect all profiles with their planes
                let mut profiles_with_planes = Vec::new();
                for sketch_id in profile_sketch_ids {
                    let sketch =
                        sketches
                            .get(sketch_id)
                            .ok_or(FeatureError::InvalidFeature(format!(
                                "Sketch {} not found",
                                sketch_id
                            )))?;

                    let sketch_profiles = sketch.extract_profiles()?;
                    if sketch_profiles.is_empty() {
                        return Err(FeatureError::InvalidFeature(format!(
                            "No closed profiles found in sketch {}",
                            sketch_id
                        )));
                    }

                    profiles_with_planes.push((
                        sketch_profiles[0].clone(),
                        sketch.plane.origin,
                        sketch.plane.normal,
                    ));
                }

                let mut solid = kernel.loft(&profiles_with_planes, *create_solid, *ruled)?;

                // Apply boolean operation
                if let (Some(op), Some(target_id)) =
                    (Option::<BooleanType>::from(*boolean_op), target_body)
                    && let Some(target) = existing_bodies.get(target_id)
                {
                    solid = kernel.boolean(target, &solid, op)?;
                }

                Ok(solid)
            }
        }
    }
}

/// A body produced by features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CadBody {
    /// Unique identifier
    pub id: Uuid,
    /// Name of the body
    pub name: String,
    /// The solid geometry (not serialized)
    #[serde(skip)]
    pub solid: Option<Solid>,
    /// Cached tessellation
    #[serde(skip)]
    pub mesh_cache: Option<TessellatedMesh>,
    /// Feature that created this body
    pub source_feature: Option<Uuid>,
}

impl Default for CadBody {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::from("Body"),
            solid: None,
            mesh_cache: None,
            source_feature: None,
        }
    }
}

impl CadBody {
    /// Create a new body with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            solid: None,
            mesh_cache: None,
            source_feature: None,
        }
    }

    /// Get the tessellated mesh, tessellating if needed
    pub fn get_mesh(&mut self, kernel: &dyn CadKernel, tolerance: f32) -> Option<&TessellatedMesh> {
        if self.mesh_cache.is_none()
            && let Some(ref solid) = self.solid
            && let Ok(mesh) = kernel.tessellate(solid, tolerance)
        {
            self.mesh_cache = Some(mesh);
        }
        self.mesh_cache.as_ref()
    }

    /// Invalidate the mesh cache
    pub fn invalidate_cache(&mut self) {
        self.mesh_cache = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_id() {
        let feature = Feature::extrude("Test", Uuid::new_v4(), 10.0, ExtrudeDirection::Positive);
        let id = feature.id();
        assert_eq!(feature.id(), id);
    }

    #[test]
    fn test_feature_suppression() {
        let mut feature =
            Feature::extrude("Test", Uuid::new_v4(), 10.0, ExtrudeDirection::Positive);
        assert!(!feature.is_suppressed());

        feature.set_suppressed(true);
        assert!(feature.is_suppressed());
    }
}
