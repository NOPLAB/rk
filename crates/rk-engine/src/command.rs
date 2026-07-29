//! Serializable commands: the only way to mutate the engine's document.
//!
//! Commands are designed to cross a protocol boundary (JSON-RPC / MCP)
//! later, so they carry no UI state and reference everything by ID.
//! Creation commands accept an optional client-generated ID; when `None`
//! the engine mints one and reports it back via events.

use std::path::PathBuf;

use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use rk_cad::{BooleanOp, ExtrudeDirection, SketchConstraint, SketchEntity, SketchPlane, Wire2D};
use rk_core::{GeometryType, InertiaMatrix, JointLimits, JointType, Pose, StlUnit};

/// A mutation of the engine document
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    // ---- Document / IO ----
    NewDocument,
    LoadDocument {
        path: PathBuf,
    },
    /// Save to `path`, or to the document's current path when `None`
    SaveDocument {
        path: Option<PathBuf>,
    },
    ImportMesh {
        path: PathBuf,
        unit: StlUnit,
    },
    /// Replaces the current document with the imported robot
    ImportUrdf {
        path: PathBuf,
        stl_unit: StlUnit,
    },
    ExportUrdf {
        path: PathBuf,
        robot_name: String,
    },
    RenameProject {
        name: String,
    },

    // ---- Part ----
    CreatePrimitive {
        id: Option<Uuid>,
        primitive: PrimitiveSpec,
        name: Option<String>,
    },
    CreateEmptyPart {
        id: Option<Uuid>,
        name: Option<String>,
    },
    DeletePart {
        part_id: Uuid,
    },
    RenamePart {
        part_id: Uuid,
        name: String,
    },
    SetPartTransform {
        part_id: Uuid,
        transform: Mat4,
    },
    SetPartColor {
        part_id: Uuid,
        color: [f32; 4],
    },
    SetPartMaterial {
        part_id: Uuid,
        material_name: Option<String>,
    },
    SetPartMass {
        part_id: Uuid,
        mass: f32,
    },
    SetPartInertia {
        part_id: Uuid,
        inertia: InertiaMatrix,
    },

    // ---- Assembly / Joint ----
    /// Connect two parts with a fixed joint, creating links as needed
    ConnectParts {
        parent_part: Uuid,
        child_part: Uuid,
    },
    DisconnectPart {
        child_part: Uuid,
    },
    SetJointPosition {
        joint_id: Uuid,
        position: f32,
    },
    ResetJointPosition {
        joint_id: Uuid,
    },
    ResetAllJointPositions,
    SetJointType {
        joint_id: Uuid,
        joint_type: JointType,
    },
    /// When `keep_child_world_pose` is true the child part's origin is
    /// compensated so its world pose does not change (gizmo semantics)
    SetJointOrigin {
        joint_id: Uuid,
        origin: Pose,
        keep_child_world_pose: bool,
    },
    SetJointAxis {
        joint_id: Uuid,
        axis: Vec3,
    },
    SetJointLimits {
        joint_id: Uuid,
        limits: Option<JointLimits>,
    },

    // ---- Collision ----
    AddCollision {
        link_id: Uuid,
        geometry: GeometryType,
        origin: Pose,
    },
    RemoveCollision {
        link_id: Uuid,
        index: usize,
    },
    SetCollisionOrigin {
        link_id: Uuid,
        index: usize,
        origin: Pose,
    },
    SetCollisionGeometry {
        link_id: Uuid,
        index: usize,
        geometry: GeometryType,
    },

    // ---- Sketch ----
    CreateSketch {
        id: Option<Uuid>,
        name: Option<String>,
        plane: SketchPlane,
    },
    DeleteSketch {
        sketch_id: Uuid,
    },
    /// Add entities atomically (a rectangle is 4 points + 4 lines in one
    /// command and therefore one undo step)
    AddSketchEntities {
        sketch_id: Uuid,
        entities: Vec<SketchEntity>,
    },
    /// Replace an entity with the same ID (move a point, resize a circle)
    UpdateSketchEntity {
        sketch_id: Uuid,
        entity: SketchEntity,
    },
    DeleteSketchEntities {
        sketch_id: Uuid,
        entity_ids: Vec<Uuid>,
    },
    AddSketchConstraint {
        sketch_id: Uuid,
        constraint: SketchConstraint,
    },
    DeleteSketchConstraint {
        sketch_id: Uuid,
        constraint_id: Uuid,
    },
    SolveSketch {
        sketch_id: Uuid,
    },
    /// Construction geometry guides the sketch without enclosing any region —
    /// centrelines, revolve axes, the circle a polygon is inscribed in
    SetSketchConstruction {
        sketch_id: Uuid,
        entity_ids: Vec<Uuid>,
        construction: bool,
    },

    // ---- Feature ----
    AddExtrude {
        id: Option<Uuid>,
        name: Option<String>,
        sketch_id: Uuid,
        /// Regions to extrude, by `Profile::id`; empty means every region the
        /// sketch encloses
        #[serde(default)]
        profiles: Vec<Uuid>,
        distance: f32,
        direction: ExtrudeDirection,
        boolean_op: BooleanOp,
        target_body: Option<Uuid>,
    },
    AddRevolve {
        id: Option<Uuid>,
        name: Option<String>,
        sketch_id: Uuid,
        /// Regions to revolve, by `Profile::id`; empty means every region
        #[serde(default)]
        profiles: Vec<Uuid>,
        axis_origin: Vec3,
        axis_direction: Vec3,
        angle: f32,
        boolean_op: BooleanOp,
        target_body: Option<Uuid>,
    },
    DeleteFeature {
        feature_id: Uuid,
    },
    SetFeatureSuppressed {
        feature_id: Uuid,
        suppressed: bool,
    },
    /// Roll back to just after the given feature (`None` = to the end)
    RollbackTo {
        feature_id: Option<Uuid>,
    },
    RebuildFeatures,

    // ---- History ----
    Undo,
    Redo,
}

/// Parametrized primitive shapes
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum PrimitiveSpec {
    Box { size: [f32; 3] },
    Cylinder { radius: f32, height: f32 },
    Sphere { radius: f32 },
}

impl PrimitiveSpec {
    pub fn type_name(&self) -> &'static str {
        match self {
            PrimitiveSpec::Box { .. } => "Box",
            PrimitiveSpec::Cylinder { .. } => "Cylinder",
            PrimitiveSpec::Sphere { .. } => "Sphere",
        }
    }
}

/// Input for [`crate::Engine::preview_extrude`] (a query, not a command)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtrudePreviewRequest {
    pub sketch_id: Uuid,
    /// Profiles to extrude (as selected in the dialog / by the agent)
    pub profiles: Vec<Wire2D>,
    pub distance: f32,
    pub direction: ExtrudeDirection,
}

impl Command {
    /// Whether applying this command records an undo snapshot
    pub(crate) fn takes_snapshot(&self) -> bool {
        !matches!(
            self,
            // No document mutation
            Command::SaveDocument { .. }
                | Command::ExportUrdf { .. }
                // Resets the whole document and clears history instead
                | Command::NewDocument
                | Command::LoadDocument { .. }
                // High-frequency runtime state (slider)
                | Command::SetJointPosition { .. }
                // Regenerates derived geometry only
                | Command::RebuildFeatures
                // Handled by the history mechanism itself
                | Command::Undo
                | Command::Redo
        )
    }

    /// Whether this command changes the document's modified flag,
    /// and to what value (`None` = leave unchanged)
    pub(crate) fn marks_modified(&self) -> Option<bool> {
        match self {
            Command::NewDocument | Command::LoadDocument { .. } | Command::SaveDocument { .. } => {
                Some(false)
            }
            Command::ExportUrdf { .. }
            | Command::SetJointPosition { .. }
            | Command::ResetJointPosition { .. }
            | Command::ResetAllJointPositions
            | Command::RebuildFeatures
            | Command::Undo
            | Command::Redo => None,
            _ => Some(true),
        }
    }

    /// Human-readable description (shown in the undo menu)
    pub fn description(&self) -> &'static str {
        match self {
            Command::NewDocument => "New Project",
            Command::LoadDocument { .. } => "Load Project",
            Command::SaveDocument { .. } => "Save Project",
            Command::ImportMesh { .. } => "Import Mesh",
            Command::ImportUrdf { .. } => "Import URDF",
            Command::ExportUrdf { .. } => "Export URDF",
            Command::RenameProject { .. } => "Rename Project",
            Command::CreatePrimitive { .. } => "Create Primitive",
            Command::CreateEmptyPart { .. } => "Create Empty Part",
            Command::DeletePart { .. } => "Delete Part",
            Command::RenamePart { .. } => "Rename Part",
            Command::SetPartTransform { .. } => "Move Part",
            Command::SetPartColor { .. } => "Change Part Color",
            Command::SetPartMaterial { .. } => "Change Part Material",
            Command::SetPartMass { .. } => "Change Part Mass",
            Command::SetPartInertia { .. } => "Change Part Inertia",
            Command::ConnectParts { .. } => "Connect Parts",
            Command::DisconnectPart { .. } => "Disconnect Part",
            Command::SetJointPosition { .. } => "Update Joint Position",
            Command::ResetJointPosition { .. } => "Reset Joint Position",
            Command::ResetAllJointPositions => "Reset All Joint Positions",
            Command::SetJointType { .. } => "Change Joint Type",
            Command::SetJointOrigin { .. } => "Update Joint Origin",
            Command::SetJointAxis { .. } => "Update Joint Axis",
            Command::SetJointLimits { .. } => "Update Joint Limits",
            Command::AddCollision { .. } => "Add Collision",
            Command::RemoveCollision { .. } => "Remove Collision",
            Command::SetCollisionOrigin { .. } => "Update Collision Origin",
            Command::SetCollisionGeometry { .. } => "Update Collision Geometry",
            Command::CreateSketch { .. } => "Create Sketch",
            Command::DeleteSketch { .. } => "Delete Sketch",
            Command::AddSketchEntities { .. } => "Add Sketch Entities",
            Command::UpdateSketchEntity { .. } => "Edit Sketch Entity",
            Command::DeleteSketchEntities { .. } => "Delete Sketch Entities",
            Command::AddSketchConstraint { .. } => "Add Constraint",
            Command::DeleteSketchConstraint { .. } => "Delete Constraint",
            Command::SolveSketch { .. } => "Solve Sketch",
            Command::SetSketchConstruction { construction, .. } => {
                if *construction {
                    "Make Construction"
                } else {
                    "Make Normal"
                }
            }
            Command::AddExtrude { .. } => "Extrude",
            Command::AddRevolve { .. } => "Revolve",
            Command::DeleteFeature { .. } => "Delete Feature",
            Command::SetFeatureSuppressed { .. } => "Suppress Feature",
            Command::RollbackTo { .. } => "Rollback History",
            Command::RebuildFeatures => "Rebuild Features",
            Command::Undo => "Undo",
            Command::Redo => "Redo",
        }
    }
}
