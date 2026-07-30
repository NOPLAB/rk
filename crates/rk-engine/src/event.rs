//! Events emitted by the engine after a command is applied.
//!
//! Events carry IDs, not bulk data: meshes and geometry are pulled from
//! the engine by ID so events stay cheap to journal and to serialize
//! across a protocol boundary. `DocumentReset` is the one coarse event —
//! it asks clients to drop everything and re-pull the whole document.

use std::path::PathBuf;

use glam::Mat4;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Why a `DocumentReset` happened
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetReason {
    New,
    Loaded,
    UrdfImported,
    UndoRedo,
}

/// A change notification from the engine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    // ---- Coarse ----
    /// The whole document was replaced; clients must re-pull everything
    /// (parts, transforms, bodies) and reset their own document-scoped state
    DocumentReset {
        reason: ResetReason,
    },
    DocumentSaved {
        path: PathBuf,
    },
    ModifiedChanged {
        modified: bool,
    },

    // ---- Part ----
    PartAdded {
        part_id: Uuid,
    },
    PartRemoved {
        part_id: Uuid,
    },
    PartRenamed {
        part_id: Uuid,
        name: String,
    },
    PartAppearanceChanged {
        part_id: Uuid,
    },
    PartPhysicsChanged {
        part_id: Uuid,
    },

    /// Final render transforms (link world transform × part origin) after a
    /// kinematics update. Renderers apply these verbatim, per part ID.
    WorldTransformsChanged {
        transforms: Vec<(Uuid, Mat4)>,
    },

    // ---- Assembly ----
    LinkAdded {
        link_id: Uuid,
        part_id: Option<Uuid>,
    },
    JointAdded {
        joint_id: Uuid,
        parent_link: Uuid,
        child_link: Uuid,
    },
    JointRemoved {
        joint_id: Uuid,
    },
    /// Type / origin / axis / limits changed
    JointChanged {
        joint_id: Uuid,
    },
    JointPositionChanged {
        joint_id: Uuid,
        position: f32,
    },

    // ---- Collision ----
    CollisionAdded {
        link_id: Uuid,
        index: usize,
    },
    CollisionRemoved {
        link_id: Uuid,
        index: usize,
    },
    CollisionChanged {
        link_id: Uuid,
        index: usize,
    },

    // ---- Sketch ----
    SketchAdded {
        sketch_id: Uuid,
    },
    SketchRemoved {
        sketch_id: Uuid,
    },
    SketchRenamed {
        sketch_id: Uuid,
        name: String,
    },
    /// Entities or constraints changed; 2D render data is pulled per frame
    SketchGeometryChanged {
        sketch_id: Uuid,
    },
    SketchSolved {
        sketch_id: Uuid,
    },

    // ---- Feature / Body ----
    FeatureAdded {
        feature_id: Uuid,
    },
    FeatureRemoved {
        feature_id: Uuid,
    },
    /// Name, suppression or rollback state changed
    FeatureChanged {
        feature_id: Uuid,
    },
    /// Feature grouping changed. Groups are browser presentation only and
    /// carry no geometry, so clients re-pull the whole list rather than
    /// tracking individual groups.
    FeatureGroupsChanged,
    /// Bodies were regenerated; renderers clear CAD bodies and re-pull
    /// each listed body's mesh via `Engine::body_mesh`
    BodiesRebuilt {
        body_ids: Vec<Uuid>,
    },

    // ---- History ----
    HistoryChanged {
        can_undo: bool,
        can_redo: bool,
        undo_description: Option<String>,
    },
}
