//! Application state module

mod editor;
mod viewport;

pub use editor::{EditorTool, PrimitiveType};
pub use viewport::{
    GizmoInteraction, GizmoTransform, PickablePartData, SharedViewportState, ViewportState,
    pick_object,
};

use std::path::PathBuf;
use std::sync::Arc;

use glam::Mat4;
use parking_lot::Mutex;
use uuid::Uuid;

use rk_core::{Part, Project, StlUnit};

/// Actions that can be performed on the app state
#[derive(Debug, Clone)]
pub enum AppAction {
    // File actions (path-based, native only)
    /// Import an STL file
    ImportStl(PathBuf),
    /// Import a URDF file
    ImportUrdf(PathBuf),
    /// Save project
    SaveProject(Option<PathBuf>),
    /// Load project
    LoadProject(PathBuf),
    /// Export URDF with path and robot name
    ExportUrdf { path: PathBuf, robot_name: String },
    /// New project
    NewProject,

    // File actions (bytes-based, for WASM)
    /// Import an STL from bytes
    ImportStlBytes { name: String, data: Vec<u8> },
    /// Load project from bytes
    LoadProjectBytes { name: String, data: Vec<u8> },

    // Part actions
    /// Create a primitive shape
    CreatePrimitive {
        primitive_type: PrimitiveType,
        name: Option<String>,
    },
    /// Create an empty part (no geometry)
    CreateEmpty { name: Option<String> },
    /// Select a part
    SelectPart(Option<Uuid>),
    /// Delete selected part
    DeleteSelectedPart,
    /// Update part transform
    UpdatePartTransform { part_id: Uuid, transform: Mat4 },

    // Assembly actions
    /// Connect two parts
    ConnectParts { parent: Uuid, child: Uuid },
    /// Disconnect a part from its parent
    DisconnectPart { child: Uuid },

    // Joint position actions
    /// Update a joint position (value in radians for revolute, meters for prismatic)
    UpdateJointPosition { joint_id: Uuid, position: f32 },
    /// Reset a joint position to 0
    ResetJointPosition { joint_id: Uuid },
    /// Reset all joint positions to 0
    ResetAllJointPositions,
}

/// Angle display mode for joint sliders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum AngleDisplayMode {
    #[default]
    Degrees,
    Radians,
}

/// Application state
pub struct AppState {
    /// Current project (contains parts, assembly, materials)
    pub project: Project,
    /// Currently selected part
    pub selected_part: Option<Uuid>,
    /// Hovered part
    pub hovered_part: Option<Uuid>,
    /// Current editor tool
    pub current_tool: EditorTool,
    /// Symmetry mode enabled
    pub symmetry_mode: bool,
    /// Project file path
    pub project_path: Option<PathBuf>,
    /// Has unsaved changes
    pub modified: bool,
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

impl Default for AppState {
    fn default() -> Self {
        Self {
            project: Project::default(),
            selected_part: None,
            hovered_part: None,
            current_tool: EditorTool::default(),
            symmetry_mode: false,
            project_path: None,
            modified: false,
            pending_actions: Vec::new(),
            show_part_axes: true,
            show_joint_markers: true,
            stl_import_unit: StlUnit::Millimeters,
            angle_display_mode: AngleDisplayMode::default(),
        }
    }
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
    /// Create a new app state
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a part (delegates to project)
    pub fn add_part(&mut self, part: Part) {
        self.project.add_part(part);
        self.modified = true;
    }

    /// Get a part by ID (delegates to project)
    pub fn get_part(&self, id: Uuid) -> Option<&Part> {
        self.project.get_part(id)
    }

    /// Get a mutable part by ID (delegates to project)
    pub fn get_part_mut(&mut self, id: Uuid) -> Option<&mut Part> {
        self.modified = true;
        self.project.get_part_mut(id)
    }

    /// Remove a part (delegates to project)
    pub fn remove_part(&mut self, id: Uuid) -> Option<Part> {
        self.modified = true;
        self.project.remove_part(id)
    }

    /// Select a part
    pub fn select_part(&mut self, id: Option<Uuid>) {
        self.selected_part = id;
    }

    /// Queue an action
    pub fn queue_action(&mut self, action: AppAction) {
        self.pending_actions.push(action);
    }

    /// Take pending actions
    pub fn take_pending_actions(&mut self) -> Vec<AppAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Reset to a new project
    pub fn new_project(&mut self) {
        self.project = Project::default();
        self.selected_part = None;
        self.project_path = None;
        self.modified = false;
    }

    /// Load a project
    pub fn load_project(&mut self, project: Project, path: PathBuf) {
        self.project = project;
        self.project_path = Some(path);
        self.selected_part = None;
        self.modified = false;
    }
}

pub type SharedAppState = Arc<Mutex<AppState>>;

/// Create a new shared app state
pub fn create_shared_state() -> SharedAppState {
    Arc::new(Mutex::new(AppState::new()))
}
