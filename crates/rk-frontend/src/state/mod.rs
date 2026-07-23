//! Application state module
//!
//! Domain state (project, CAD data, undo history) lives in the engine
//! (`rk_engine::Engine`); this module holds only UI state — selection,
//! editor mode, tools, dialogs — plus the action queue.

mod editor;
mod sketch;
mod viewport;

pub use editor::{EditorTool, PrimitiveType};
pub use sketch::{
    ConstraintToolState, DimensionDialogState, EditorMode, ExtrudeDialogState, ExtrudeDirection,
    InProgressEntity, PlaneSelectionState, ReferencePlane, SketchModeState, SketchTool,
    SketchUiAction,
};
pub use viewport::{
    GizmoInteraction, GizmoTransform, PickablePartData, SharedViewportState, ViewportState,
    pick_object,
};

use std::sync::Arc;

use parking_lot::Mutex;
use uuid::Uuid;

use rk_core::StlUnit;
use rk_engine::{Command, InteractionId, SharedEngine};

/// Actions queued by UI panels and processed once per frame
#[derive(Debug, Clone)]
pub enum AppAction {
    // ---- UI-only (no engine involvement) ----
    /// Select a part
    SelectPart(Option<Uuid>),
    /// Select a collision element (link_id, collision_index)
    SelectCollision(Option<(Uuid, usize)>),
    /// Set the joint being edited (for gizmo display)
    SetEditingJoint(Option<Uuid>),
    /// Sketch-mode UI actions (tools, dialogs, mode transitions)
    SketchUi(SketchUiAction),

    // ---- Engine ----
    /// Forward a command to the engine
    Cmd(Command),
    /// Apply a command as part of a drag/edit session (coalesced undo)
    Interactive {
        session: InteractionId,
        cmd: Command,
    },
    /// End a drag/edit session
    EndInteraction {
        session: InteractionId,
        cancel: bool,
    },

    // ---- Mixed UI + engine operations ----
    Composite(CompositeAction),
}

/// Operations that combine engine commands with UI state changes
#[derive(Debug, Clone)]
pub enum CompositeAction {
    /// Move camera, create a sketch on the plane, and enter sketch mode
    SelectPlaneAndCreateSketch { plane: ReferencePlane },
    /// Solve the active sketch and return to assembly mode
    ExitSketchMode,
    /// Delete the selected sketch entities and clear the selection
    DeleteSelectedSketchEntities,
    /// Run the extrusion configured in the extrude dialog
    ExecuteExtrude,
    /// Add the dimension constraint configured in the dimension dialog
    ConfirmDimensionConstraint,
    /// Exit sketch mode if needed, then delete the sketch
    DeleteSketch { sketch_id: Uuid },
}

/// Angle display mode for joint sliders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum AngleDisplayMode {
    #[default]
    Degrees,
    Radians,
}

/// UI state (the engine owns all domain state)
pub struct AppState {
    /// Handle to the engine. Lock briefly, copy what you need, release —
    /// never hold this guard while locking the viewport.
    pub engine: SharedEngine,
    /// Editor mode (assembly / plane selection / sketch)
    pub editor_mode: EditorMode,
    /// Currently selected part
    pub selected_part: Option<Uuid>,
    /// Currently selected collision element (link_id, collision_index)
    pub selected_collision: Option<(Uuid, usize)>,
    /// Currently editing joint (for gizmo display)
    pub editing_joint_id: Option<Uuid>,
    /// Hovered part
    pub hovered_part: Option<Uuid>,
    /// Current editor tool
    pub current_tool: EditorTool,
    /// Symmetry mode enabled
    pub symmetry_mode: bool,
    /// Pending actions
    pending_actions: Vec<AppAction>,
    /// Show axes on selected part
    pub show_part_axes: bool,
    /// Show joint markers
    pub show_joint_markers: bool,
    /// Global unit setting for STL import and other operations
    pub stl_import_unit: StlUnit,
    /// Angle display mode for joint sliders
    pub angle_display_mode: AngleDisplayMode,
}

impl AngleDisplayMode {
    /// Toggle between degrees and radians
    pub fn toggle(&mut self) {
        *self = match self {
            AngleDisplayMode::Degrees => AngleDisplayMode::Radians,
            AngleDisplayMode::Radians => AngleDisplayMode::Degrees,
        };
    }

    /// Convert radians to display value
    pub fn from_radians(&self, radians: f32) -> f32 {
        match self {
            AngleDisplayMode::Degrees => radians.to_degrees(),
            AngleDisplayMode::Radians => radians,
        }
    }

    /// Convert display value to radians
    pub fn to_radians(&self, value: f32) -> f32 {
        match self {
            AngleDisplayMode::Degrees => value.to_radians(),
            AngleDisplayMode::Radians => value,
        }
    }

    /// Get the suffix for display
    pub fn suffix(&self) -> &'static str {
        match self {
            AngleDisplayMode::Degrees => "\u{00b0}",
            AngleDisplayMode::Radians => " rad",
        }
    }
}

impl AppState {
    /// Create a new UI state bound to an engine
    pub fn new(engine: SharedEngine) -> Self {
        Self {
            engine,
            editor_mode: EditorMode::default(),
            selected_part: None,
            selected_collision: None,
            editing_joint_id: None,
            hovered_part: None,
            current_tool: EditorTool::default(),
            symmetry_mode: false,
            pending_actions: Vec::new(),
            show_part_axes: true,
            show_joint_markers: true,
            stl_import_unit: StlUnit::Millimeters,
            angle_display_mode: AngleDisplayMode::default(),
        }
    }

    /// Select a part
    pub fn select_part(&mut self, id: Option<Uuid>) {
        self.selected_part = id;
        // Clear joint editing when selecting a part
        self.editing_joint_id = None;
    }

    /// Clear all selections (after a document reset)
    pub fn clear_selections(&mut self) {
        self.selected_part = None;
        self.selected_collision = None;
        self.editing_joint_id = None;
        self.hovered_part = None;
    }

    /// Queue an action
    pub fn queue_action(&mut self, action: AppAction) {
        self.pending_actions.push(action);
    }

    /// Take pending actions
    pub fn take_pending_actions(&mut self) -> Vec<AppAction> {
        std::mem::take(&mut self.pending_actions)
    }
}

pub type SharedAppState = Arc<Mutex<AppState>>;

/// Create a new shared app state bound to an engine
pub fn create_shared_state(engine: SharedEngine) -> SharedAppState {
    Arc::new(Mutex::new(AppState::new(engine)))
}
