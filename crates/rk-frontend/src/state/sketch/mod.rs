//! Sketch mode state types

mod dialogs;
mod entities;
mod mode_state;
mod tools;

pub use dialogs::{DimensionDialogState, ExtrudeDialogState, ExtrudeDirection};
pub use entities::{ConstraintToolState, InProgressEntity};
pub use mode_state::SketchModeState;
pub use tools::SketchTool;

use glam::Vec3;
use uuid::Uuid;

use rk_cad::{BooleanOp, SketchPlane};

/// Reference plane types for sketch creation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferencePlane {
    /// XY plane (Top view) - Z axis normal
    XY,
    /// XZ plane (Front view) - Y axis normal
    XZ,
    /// YZ plane (Side view) - X axis normal
    YZ,
}

impl ReferencePlane {
    /// Get the normal vector of the plane
    pub fn normal(&self) -> Vec3 {
        match self {
            ReferencePlane::XY => Vec3::Z,
            ReferencePlane::XZ => Vec3::Y,
            ReferencePlane::YZ => Vec3::X,
        }
    }

    /// Convert to a SketchPlane
    pub fn to_sketch_plane(&self) -> SketchPlane {
        match self {
            ReferencePlane::XY => SketchPlane::xy(),
            ReferencePlane::XZ => SketchPlane::xz(),
            ReferencePlane::YZ => SketchPlane::yz(),
        }
    }

    /// Get the display name of the plane
    pub fn name(&self) -> &'static str {
        match self {
            ReferencePlane::XY => "XY (Top)",
            ReferencePlane::XZ => "XZ (Front)",
            ReferencePlane::YZ => "YZ (Side)",
        }
    }

    /// Get all reference planes
    pub fn all() -> [ReferencePlane; 3] {
        [ReferencePlane::XY, ReferencePlane::XZ, ReferencePlane::YZ]
    }
}

/// State for plane selection mode
#[derive(Debug, Clone, Default)]
pub struct PlaneSelectionState {
    /// Currently hovered plane (for highlighting)
    pub hovered_plane: Option<ReferencePlane>,
}

/// Editor mode (3D assembly or 2D sketch)
#[derive(Debug, Clone, Default)]
#[allow(clippy::large_enum_variant)]
pub enum EditorMode {
    /// Normal 3D assembly editing
    #[default]
    Assembly,
    /// Plane selection mode (before entering sketch mode)
    PlaneSelection(PlaneSelectionState),
    /// 2D sketch editing mode
    Sketch(SketchModeState),
}

impl EditorMode {
    /// Check if in sketch mode
    pub fn is_sketch(&self) -> bool {
        matches!(self, EditorMode::Sketch(_))
    }

    /// Check if in plane selection mode
    pub fn is_plane_selection(&self) -> bool {
        matches!(self, EditorMode::PlaneSelection(_))
    }

    /// Get sketch mode state if in sketch mode
    pub fn sketch(&self) -> Option<&SketchModeState> {
        match self {
            EditorMode::Sketch(state) => Some(state),
            _ => None,
        }
    }

    /// Get mutable sketch mode state if in sketch mode
    pub fn sketch_mut(&mut self) -> Option<&mut SketchModeState> {
        match self {
            EditorMode::Sketch(state) => Some(state),
            _ => None,
        }
    }

    /// Get plane selection state if in plane selection mode
    pub fn plane_selection(&self) -> Option<&PlaneSelectionState> {
        match self {
            EditorMode::PlaneSelection(state) => Some(state),
            _ => None,
        }
    }

    /// Get mutable plane selection state if in plane selection mode
    pub fn plane_selection_mut(&mut self) -> Option<&mut PlaneSelectionState> {
        match self {
            EditorMode::PlaneSelection(state) => Some(state),
            _ => None,
        }
    }
}

/// UI-only sketch actions (mode transitions, tools, dialog state).
/// Domain changes (entities, constraints, features) go through
/// `rk_engine::Command` instead.
#[derive(Debug, Clone)]
pub enum SketchUiAction {
    /// Begin plane selection mode (before creating a sketch)
    BeginPlaneSelection,
    /// Cancel plane selection and return to assembly mode
    CancelPlaneSelection,
    /// Update hovered plane during plane selection
    SetHoveredPlane { plane: Option<ReferencePlane> },
    /// Enter sketch editing mode for an existing sketch
    EditSketch { sketch_id: Uuid },
    /// Set the current tool
    SetTool { tool: SketchTool },
    /// Toggle grid snapping
    ToggleSnap,
    /// Set grid spacing
    SetGridSpacing { spacing: f32 },
    /// Show the extrude dialog for the current sketch
    ShowExtrudeDialog,
    /// Update extrude dialog distance
    UpdateExtrudeDistance { distance: f32 },
    /// Update extrude dialog direction
    UpdateExtrudeDirection { direction: ExtrudeDirection },
    /// Update extrude dialog boolean operation
    UpdateExtrudeBooleanOp { boolean_op: BooleanOp },
    /// Update extrude dialog target body
    UpdateExtrudeTargetBody { target_body: Option<Uuid> },
    /// Toggle a profile selection in the extrude dialog
    ToggleExtrudeProfile { profile_index: usize },
    /// Cancel the extrude dialog
    CancelExtrudeDialog,
    /// Select an entity for constraint tool
    SelectEntityForConstraint { entity_id: Uuid },
    /// Cancel constraint tool selection
    CancelConstraintSelection,
    /// Show dimension input dialog
    ShowDimensionDialog {
        tool: SketchTool,
        entities: Vec<Uuid>,
        initial_value: f32,
    },
    /// Update dimension dialog value
    UpdateDimensionValue { value: f32 },
    /// Cancel dimension dialog
    CancelDimensionDialog,
}
