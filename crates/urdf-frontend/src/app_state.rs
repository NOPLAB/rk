//! Application state

use std::collections::HashMap;
use std::path::PathBuf;

use parking_lot::Mutex;
use std::sync::Arc;
use uuid::Uuid;

use urdf_core::{Part, Project, StlUnit};

/// Editor tool mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorTool {
    #[default]
    Select,
    Move,
    Rotate,
    AddJointPoint,
}

impl EditorTool {
    pub fn name(&self) -> &'static str {
        match self {
            EditorTool::Select => "Select",
            EditorTool::Move => "Move",
            EditorTool::Rotate => "Rotate",
            EditorTool::AddJointPoint => "Add Joint Point",
        }
    }
}

/// Application state
pub struct AppState {
    /// Current project
    pub project: Project,
    /// Parts indexed by ID for quick lookup
    pub parts: HashMap<Uuid, Part>,
    /// Currently selected part
    pub selected_part: Option<Uuid>,
    /// Currently selected joint point (part_id, point_index)
    pub selected_joint_point: Option<(Uuid, usize)>,
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
    pub pending_actions: Vec<AppAction>,
    /// Show axes on selected part
    pub show_part_axes: bool,
    /// Show joint point markers
    pub show_joint_markers: bool,
    /// STL import unit (for scaling)
    pub stl_import_unit: StlUnit,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            project: Project::default(),
            parts: HashMap::new(),
            selected_part: None,
            selected_joint_point: None,
            hovered_part: None,
            current_tool: EditorTool::default(),
            symmetry_mode: false,
            project_path: None,
            modified: false,
            pending_actions: Vec::new(),
            show_part_axes: true,
            show_joint_markers: true,
            stl_import_unit: StlUnit::Millimeters,
        }
    }
}

impl AppState {
    /// Create a new app state
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a part
    pub fn add_part(&mut self, part: Part) {
        let id = part.id;
        self.parts.insert(id, part.clone());
        self.project.add_part(part);
        self.modified = true;
    }

    /// Get a part by ID
    pub fn get_part(&self, id: Uuid) -> Option<&Part> {
        self.parts.get(&id)
    }

    /// Get a mutable part by ID
    pub fn get_part_mut(&mut self, id: Uuid) -> Option<&mut Part> {
        self.modified = true;
        self.parts.get_mut(&id)
    }

    /// Remove a part
    pub fn remove_part(&mut self, id: Uuid) -> Option<Part> {
        self.modified = true;
        self.project.remove_part(id);
        self.parts.remove(&id)
    }

    /// Select a part
    pub fn select_part(&mut self, id: Option<Uuid>) {
        self.selected_part = id;
        self.selected_joint_point = None;
    }

    /// Select a joint point
    pub fn select_joint_point(&mut self, part_id: Uuid, point_idx: usize) {
        self.selected_part = Some(part_id);
        self.selected_joint_point = Some((part_id, point_idx));
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
        self.parts.clear();
        self.selected_part = None;
        self.selected_joint_point = None;
        self.project_path = None;
        self.modified = false;
    }

    /// Load a project
    pub fn load_project(&mut self, project: Project, path: PathBuf) {
        self.parts.clear();
        for part in &project.parts {
            self.parts.insert(part.id, part.clone());
        }
        self.project = project;
        self.project_path = Some(path);
        self.selected_part = None;
        self.selected_joint_point = None;
        self.modified = false;
    }
}

/// Primitive type for creating geometric shapes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Box,
    Cylinder,
    Sphere,
}

impl PrimitiveType {
    pub fn name(&self) -> &'static str {
        match self {
            PrimitiveType::Box => "Box",
            PrimitiveType::Cylinder => "Cylinder",
            PrimitiveType::Sphere => "Sphere",
        }
    }
}

/// Actions that can be performed on the app state
#[derive(Debug, Clone)]
pub enum AppAction {
    /// Import an STL file
    ImportStl(PathBuf),
    /// Create a primitive shape
    CreatePrimitive { primitive_type: PrimitiveType, name: Option<String> },
    /// Create an empty part (no geometry)
    CreateEmpty { name: Option<String> },
    /// Select a part
    SelectPart(Option<Uuid>),
    /// Delete selected part
    DeleteSelectedPart,
    /// Update part transform
    UpdatePartTransform { part_id: Uuid, transform: glam::Mat4 },
    /// Add a joint point to a part
    AddJointPoint { part_id: Uuid, position: glam::Vec3 },
    /// Remove a joint point
    RemoveJointPoint { part_id: Uuid, point_id: Uuid },
    /// Connect two parts
    ConnectParts { parent: Uuid, child: Uuid },
    /// Disconnect a part from its parent
    DisconnectPart { child: Uuid },
    /// Export URDF with path and robot name
    ExportUrdf { path: PathBuf, robot_name: String },
    /// Save project
    SaveProject(Option<PathBuf>),
    /// Load project
    LoadProject(PathBuf),
    /// New project
    NewProject,
    /// Connect a part to base_link
    ConnectToBaseLink(Uuid),
    /// Import a URDF file
    ImportUrdf(PathBuf),
}

pub type SharedAppState = Arc<Mutex<AppState>>;

/// Create a new shared app state
pub fn create_shared_state() -> SharedAppState {
    Arc::new(Mutex::new(AppState::new()))
}
