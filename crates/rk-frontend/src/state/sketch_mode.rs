//! Sketch mode state types

use std::collections::HashSet;

use glam::{Vec2, Vec3};
use uuid::Uuid;

use rk_cad::{
    BooleanOp, CadData, Sketch, SketchConstraint, SketchEntity, SketchPlane, TessellatedMesh,
    Wire2D,
};

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

/// Tool for sketch editing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SketchTool {
    /// Select and move entities
    #[default]
    Select,
    /// Draw a point
    Point,
    /// Draw a line
    Line,
    /// Draw a rectangle from corner to corner
    RectangleCorner,
    /// Draw a rectangle from center outward
    RectangleCenter,
    /// Draw a rectangle from 3 points (corner, corner, height)
    Rectangle3Point,
    /// Draw a circle from center and radius
    CircleCenterRadius,
    /// Draw a circle from 2 points (diameter endpoints)
    Circle2Point,
    /// Draw a circle from 3 points on the circumference
    Circle3Point,
    /// Draw an arc from center, start angle, and end angle
    ArcCenterStartEnd,
    /// Draw an arc from 3 points (start, midpoint, end)
    Arc3Point,
    /// Add coincident constraint
    ConstrainCoincident,
    /// Add horizontal constraint
    ConstrainHorizontal,
    /// Add vertical constraint
    ConstrainVertical,
    /// Add parallel constraint
    ConstrainParallel,
    /// Add perpendicular constraint
    ConstrainPerpendicular,
    /// Add tangent constraint
    ConstrainTangent,
    /// Add equal length/radius constraint
    ConstrainEqual,
    /// Fix entity position
    ConstrainFixed,
    /// Add distance dimension
    DimensionDistance,
    /// Add horizontal distance dimension
    DimensionHorizontal,
    /// Add vertical distance dimension
    DimensionVertical,
    /// Add angle dimension
    DimensionAngle,
    /// Add radius dimension
    DimensionRadius,
}

impl SketchTool {
    /// Get the display name of the tool
    pub fn name(&self) -> &'static str {
        match self {
            SketchTool::Select => "Select",
            SketchTool::Point => "Point",
            SketchTool::Line => "Line",
            SketchTool::RectangleCorner => "Rectangle (Corner)",
            SketchTool::RectangleCenter => "Rectangle (Center)",
            SketchTool::Rectangle3Point => "Rectangle (3 Point)",
            SketchTool::CircleCenterRadius => "Circle (Center)",
            SketchTool::Circle2Point => "Circle (2 Point)",
            SketchTool::Circle3Point => "Circle (3 Point)",
            SketchTool::ArcCenterStartEnd => "Arc (Center)",
            SketchTool::Arc3Point => "Arc (3 Point)",
            SketchTool::ConstrainCoincident => "Coincident",
            SketchTool::ConstrainHorizontal => "Horizontal",
            SketchTool::ConstrainVertical => "Vertical",
            SketchTool::ConstrainParallel => "Parallel",
            SketchTool::ConstrainPerpendicular => "Perpendicular",
            SketchTool::ConstrainTangent => "Tangent",
            SketchTool::ConstrainEqual => "Equal",
            SketchTool::ConstrainFixed => "Fixed",
            SketchTool::DimensionDistance => "Distance",
            SketchTool::DimensionHorizontal => "Horizontal Dim",
            SketchTool::DimensionVertical => "Vertical Dim",
            SketchTool::DimensionAngle => "Angle",
            SketchTool::DimensionRadius => "Radius",
        }
    }

    /// Get a short label for the tool (for toolbar buttons)
    pub fn short_label(&self) -> &'static str {
        match self {
            SketchTool::Select => "⬚",
            SketchTool::Point => "•",
            SketchTool::Line => "╱",
            SketchTool::RectangleCorner => "▭",
            SketchTool::RectangleCenter => "⊞",
            SketchTool::Rectangle3Point => "▱",
            SketchTool::CircleCenterRadius => "○",
            SketchTool::Circle2Point => "⊖",
            SketchTool::Circle3Point => "◎",
            SketchTool::ArcCenterStartEnd => "⌒",
            SketchTool::Arc3Point => "◠",
            SketchTool::ConstrainCoincident => "⊙",
            SketchTool::ConstrainHorizontal => "─",
            SketchTool::ConstrainVertical => "│",
            SketchTool::ConstrainParallel => "∥",
            SketchTool::ConstrainPerpendicular => "⊥",
            SketchTool::ConstrainTangent => "⌢",
            SketchTool::ConstrainEqual => "=",
            SketchTool::ConstrainFixed => "⚓",
            SketchTool::DimensionDistance => "↔",
            SketchTool::DimensionHorizontal => "⟷",
            SketchTool::DimensionVertical => "⟷",
            SketchTool::DimensionAngle => "∠",
            SketchTool::DimensionRadius => "R",
        }
    }

    /// Check if this is a drawing tool
    pub fn is_drawing(&self) -> bool {
        matches!(
            self,
            SketchTool::Point
                | SketchTool::Line
                | SketchTool::RectangleCorner
                | SketchTool::RectangleCenter
                | SketchTool::Rectangle3Point
                | SketchTool::CircleCenterRadius
                | SketchTool::Circle2Point
                | SketchTool::Circle3Point
                | SketchTool::ArcCenterStartEnd
                | SketchTool::Arc3Point
        )
    }

    /// Check if this is a constraint tool
    pub fn is_constraint(&self) -> bool {
        matches!(
            self,
            SketchTool::ConstrainCoincident
                | SketchTool::ConstrainHorizontal
                | SketchTool::ConstrainVertical
                | SketchTool::ConstrainParallel
                | SketchTool::ConstrainPerpendicular
                | SketchTool::ConstrainTangent
                | SketchTool::ConstrainEqual
                | SketchTool::ConstrainFixed
                | SketchTool::DimensionDistance
                | SketchTool::DimensionHorizontal
                | SketchTool::DimensionVertical
                | SketchTool::DimensionAngle
                | SketchTool::DimensionRadius
        )
    }

    /// Check if this is a dimension tool
    pub fn is_dimension(&self) -> bool {
        matches!(
            self,
            SketchTool::DimensionDistance
                | SketchTool::DimensionHorizontal
                | SketchTool::DimensionVertical
                | SketchTool::DimensionAngle
                | SketchTool::DimensionRadius
        )
    }
}

/// Entity being drawn (in progress)
#[derive(Debug, Clone)]
pub enum InProgressEntity {
    /// Line from start point (awaiting end point)
    Line {
        start_point: Uuid,
        preview_end: Vec2,
    },
    /// Circle with center (awaiting radius click)
    Circle {
        center_point: Uuid,
        preview_radius: f32,
    },
    /// Arc with center (awaiting start and end points)
    Arc {
        center_point: Uuid,
        start_point: Option<Uuid>,
        preview_end: Vec2,
    },
    /// Rectangle with first corner (awaiting second corner)
    Rectangle {
        corner1: Vec2,
        preview_corner2: Vec2,
    },
}

/// State for constraint tool selection workflow
#[derive(Debug, Clone, Default)]
pub enum ConstraintToolState {
    /// Waiting for first entity selection
    #[default]
    WaitingForFirst,
    /// First entity selected, waiting for second (if needed)
    WaitingForSecond { first_entity: Uuid },
}

/// Direction for extrusion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtrudeDirection {
    /// Extrude in the positive normal direction
    #[default]
    Positive,
    /// Extrude in the negative normal direction
    Negative,
    /// Extrude symmetrically in both directions
    Symmetric,
}

impl ExtrudeDirection {
    /// Get the display name
    pub fn name(&self) -> &'static str {
        match self {
            ExtrudeDirection::Positive => "Positive",
            ExtrudeDirection::Negative => "Negative",
            ExtrudeDirection::Symmetric => "Symmetric",
        }
    }

    /// Get all direction variants
    pub fn all() -> [ExtrudeDirection; 3] {
        [
            ExtrudeDirection::Positive,
            ExtrudeDirection::Negative,
            ExtrudeDirection::Symmetric,
        ]
    }
}

/// State for the extrude dialog
#[derive(Debug, Clone)]
pub struct ExtrudeDialogState {
    /// Whether the dialog is open
    pub open: bool,
    /// ID of the sketch to extrude
    pub sketch_id: Uuid,
    /// Extrusion distance
    pub distance: f32,
    /// Extrusion direction
    pub direction: ExtrudeDirection,
    /// Extracted profiles from the sketch
    pub profiles: Vec<Wire2D>,
    /// Currently selected profile indices (supports multiple selection)
    pub selected_profile_indices: HashSet<usize>,
    /// Preview mesh for the extrusion (stored for rendering)
    pub preview_mesh: Option<TessellatedMesh>,
    /// Error message if preview generation failed
    pub error_message: Option<String>,
    /// Boolean operation type
    pub boolean_op: BooleanOp,
    /// Target body for boolean operations (None for New)
    pub target_body: Option<Uuid>,
    /// Available bodies for boolean operations (id, name)
    pub available_bodies: Vec<(Uuid, String)>,
}

impl Default for ExtrudeDialogState {
    fn default() -> Self {
        Self {
            open: false,
            sketch_id: Uuid::nil(),
            distance: 10.0,
            direction: ExtrudeDirection::Positive,
            profiles: Vec::new(),
            selected_profile_indices: HashSet::new(),
            preview_mesh: None,
            error_message: None,
            boolean_op: BooleanOp::New,
            target_body: None,
            available_bodies: Vec::new(),
        }
    }
}

impl ExtrudeDialogState {
    /// Open the dialog for a sketch (basic initialization, profiles set separately)
    pub fn open_for_sketch(&mut self, sketch_id: Uuid) {
        self.open = true;
        self.sketch_id = sketch_id;
        self.distance = 10.0;
        self.direction = ExtrudeDirection::Positive;
        self.profiles = Vec::new();
        self.selected_profile_indices = HashSet::new();
        self.preview_mesh = None;
        self.error_message = None;
        self.boolean_op = BooleanOp::New;
        self.target_body = None;
        self.available_bodies = Vec::new();
    }

    /// Set the profiles extracted from the sketch and select all by default
    pub fn set_profiles(&mut self, profiles: Vec<Wire2D>) {
        let count = profiles.len();
        self.profiles = profiles;
        // Select all profiles by default
        self.selected_profile_indices = (0..count).collect();
    }

    /// Get the currently selected profiles
    pub fn selected_profiles(&self) -> Vec<&Wire2D> {
        self.selected_profile_indices
            .iter()
            .filter_map(|&i| self.profiles.get(i))
            .collect()
    }

    /// Check if a profile is selected
    pub fn is_profile_selected(&self, index: usize) -> bool {
        self.selected_profile_indices.contains(&index)
    }

    /// Toggle profile selection by index
    pub fn toggle_profile(&mut self, index: usize) {
        if index < self.profiles.len() {
            if self.selected_profile_indices.contains(&index) {
                self.selected_profile_indices.remove(&index);
            } else {
                self.selected_profile_indices.insert(index);
            }
        }
    }

    /// Close the dialog
    pub fn close(&mut self) {
        self.open = false;
        self.profiles.clear();
        self.selected_profile_indices.clear();
        self.preview_mesh = None;
        self.error_message = None;
    }
}

/// State for the dimension input dialog
#[derive(Debug, Clone, Default)]
pub struct DimensionDialogState {
    /// Whether the dialog is open
    pub open: bool,
    /// The constraint tool being used
    pub tool: Option<SketchTool>,
    /// Selected entity IDs
    pub entities: Vec<Uuid>,
    /// The dimension value
    pub value: f32,
    /// Input field text (for editing)
    pub value_text: String,
}

impl DimensionDialogState {
    /// Open the dialog for a dimension constraint
    pub fn open_for_constraint(
        &mut self,
        tool: SketchTool,
        entities: Vec<Uuid>,
        initial_value: f32,
    ) {
        self.open = true;
        self.tool = Some(tool);
        self.entities = entities;
        self.value = initial_value;
        self.value_text = format!("{:.2}", initial_value);
    }

    /// Close the dialog
    pub fn close(&mut self) {
        self.open = false;
        self.tool = None;
        self.entities.clear();
    }
}

/// Sketch editing mode state
#[derive(Debug, Clone)]
pub struct SketchModeState {
    /// ID of the sketch being edited
    pub active_sketch: Uuid,
    /// Current drawing/editing tool
    pub current_tool: SketchTool,
    /// Entity being drawn
    pub in_progress: Option<InProgressEntity>,
    /// Selected entities
    pub selected_entities: Vec<Uuid>,
    /// Hovered entity
    pub hovered_entity: Option<Uuid>,
    /// Snap to grid
    pub snap_to_grid: bool,
    /// Grid spacing for snapping
    pub grid_spacing: f32,
    /// Extrude dialog state
    pub extrude_dialog: ExtrudeDialogState,
    /// State for constraint tool selection workflow
    pub constraint_tool_state: Option<ConstraintToolState>,
    /// Dimension input dialog state
    pub dimension_dialog: DimensionDialogState,
}

impl Default for SketchModeState {
    fn default() -> Self {
        Self {
            active_sketch: Uuid::nil(),
            current_tool: SketchTool::Select,
            in_progress: None,
            selected_entities: Vec::new(),
            hovered_entity: None,
            snap_to_grid: false,
            grid_spacing: 1.0,
            extrude_dialog: ExtrudeDialogState::default(),
            constraint_tool_state: None,
            dimension_dialog: DimensionDialogState::default(),
        }
    }
}

impl SketchModeState {
    /// Create a new sketch mode state for editing a sketch
    pub fn new(sketch_id: Uuid) -> Self {
        Self {
            active_sketch: sketch_id,
            ..Default::default()
        }
    }

    /// Clear the current selection
    pub fn clear_selection(&mut self) {
        self.selected_entities.clear();
    }

    /// Add an entity to selection
    pub fn select_entity(&mut self, id: Uuid) {
        if !self.selected_entities.contains(&id) {
            self.selected_entities.push(id);
        }
    }

    /// Remove an entity from selection
    pub fn deselect_entity(&mut self, id: Uuid) {
        self.selected_entities.retain(|&e| e != id);
    }

    /// Toggle entity selection
    pub fn toggle_selection(&mut self, id: Uuid) {
        if self.selected_entities.contains(&id) {
            self.deselect_entity(id);
        } else {
            self.select_entity(id);
        }
    }

    /// Cancel in-progress drawing
    pub fn cancel_drawing(&mut self) {
        self.in_progress = None;
    }

    /// Snap a point to grid if enabled
    pub fn snap_point(&self, point: Vec2) -> Vec2 {
        if self.snap_to_grid {
            Vec2::new(
                (point.x / self.grid_spacing).round() * self.grid_spacing,
                (point.y / self.grid_spacing).round() * self.grid_spacing,
            )
        } else {
            point
        }
    }
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

/// Actions related to sketch mode
#[derive(Debug, Clone)]
pub enum SketchAction {
    /// Begin plane selection mode (before creating a sketch)
    BeginPlaneSelection,
    /// Cancel plane selection and return to assembly mode
    CancelPlaneSelection,
    /// Select a plane and create a new sketch on it
    SelectPlaneAndCreateSketch { plane: ReferencePlane },
    /// Update hovered plane during plane selection
    SetHoveredPlane { plane: Option<ReferencePlane> },
    /// Create a new sketch on a plane (direct, without plane selection)
    CreateSketch { plane: SketchPlane },
    /// Enter sketch editing mode
    EditSketch { sketch_id: Uuid },
    /// Exit sketch editing mode
    ExitSketchMode,
    /// Set the current tool
    SetTool { tool: SketchTool },
    /// Add an entity to the sketch
    AddEntity { entity: SketchEntity },
    /// Delete selected entities
    DeleteSelected,
    /// Add a constraint
    AddConstraint { constraint: SketchConstraint },
    /// Delete a constraint
    DeleteConstraint { constraint_id: Uuid },
    /// Solve the sketch
    SolveSketch,
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
    /// Execute the extrusion with current dialog settings
    ExecuteExtrude,
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
    /// Confirm dimension and add constraint
    ConfirmDimensionConstraint,
    /// Cancel dimension dialog
    CancelDimensionDialog,
    /// Delete a sketch
    DeleteSketch { sketch_id: Uuid },
    /// Delete a feature
    DeleteFeature { feature_id: Uuid },
    /// Toggle feature suppression
    ToggleFeatureSuppression { feature_id: Uuid },
}

/// Extended CAD state for the application
#[derive(Debug, Clone, Default)]
pub struct CadState {
    /// CAD data (sketches, features, bodies)
    pub data: CadData,
    /// Current editor mode
    pub editor_mode: EditorMode,
}

impl CadState {
    /// Create a new CAD state
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new sketch on the given plane
    pub fn create_sketch(&mut self, name: impl Into<String>, plane: SketchPlane) -> Uuid {
        let sketch = Sketch::new(name, plane);
        let id = sketch.id;
        self.data.history.add_sketch(sketch);
        id
    }

    /// Get a sketch by ID
    pub fn get_sketch(&self, id: Uuid) -> Option<&Sketch> {
        self.data.history.get_sketch(id)
    }

    /// Get a mutable sketch by ID
    pub fn get_sketch_mut(&mut self, id: Uuid) -> Option<&mut Sketch> {
        self.data.history.get_sketch_mut(id)
    }

    /// Enter sketch editing mode
    pub fn enter_sketch_mode(&mut self, sketch_id: Uuid) {
        self.editor_mode = EditorMode::Sketch(SketchModeState::new(sketch_id));
    }

    /// Exit sketch editing mode
    pub fn exit_sketch_mode(&mut self) {
        // Solve the sketch before exiting
        if let EditorMode::Sketch(state) = &self.editor_mode
            && let Some(sketch) = self.data.history.get_sketch_mut(state.active_sketch)
        {
            sketch.solve();
        }
        self.editor_mode = EditorMode::Assembly;
    }

    /// Check if currently in sketch mode
    pub fn is_sketch_mode(&self) -> bool {
        self.editor_mode.is_sketch()
    }
}
