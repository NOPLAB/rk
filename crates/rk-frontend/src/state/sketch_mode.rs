//! Sketch mode state types

use glam::Vec2;
use uuid::Uuid;

use rk_cad::{CadData, Sketch, SketchConstraint, SketchEntity, SketchPlane};

/// Tool for sketch editing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SketchTool {
    /// Select and move entities
    #[default]
    Select,
    /// Draw a line
    Line,
    /// Draw a circle
    Circle,
    /// Draw an arc
    Arc,
    /// Draw a rectangle
    Rectangle,
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
    /// Add distance dimension
    DimensionDistance,
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
            SketchTool::Line => "Line",
            SketchTool::Circle => "Circle",
            SketchTool::Arc => "Arc",
            SketchTool::Rectangle => "Rectangle",
            SketchTool::ConstrainCoincident => "Coincident",
            SketchTool::ConstrainHorizontal => "Horizontal",
            SketchTool::ConstrainVertical => "Vertical",
            SketchTool::ConstrainParallel => "Parallel",
            SketchTool::ConstrainPerpendicular => "Perpendicular",
            SketchTool::DimensionDistance => "Distance",
            SketchTool::DimensionAngle => "Angle",
            SketchTool::DimensionRadius => "Radius",
        }
    }

    /// Check if this is a drawing tool
    pub fn is_drawing(&self) -> bool {
        matches!(
            self,
            SketchTool::Line | SketchTool::Circle | SketchTool::Arc | SketchTool::Rectangle
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
                | SketchTool::DimensionDistance
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
}

impl Default for SketchModeState {
    fn default() -> Self {
        Self {
            active_sketch: Uuid::nil(),
            current_tool: SketchTool::Select,
            in_progress: None,
            selected_entities: Vec::new(),
            hovered_entity: None,
            snap_to_grid: true,
            grid_spacing: 1.0,
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
pub enum EditorMode {
    /// Normal 3D assembly editing
    #[default]
    Assembly,
    /// 2D sketch editing mode
    Sketch(SketchModeState),
}

impl EditorMode {
    /// Check if in sketch mode
    pub fn is_sketch(&self) -> bool {
        matches!(self, EditorMode::Sketch(_))
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
}

/// Actions related to sketch mode
#[derive(Debug, Clone)]
pub enum SketchAction {
    /// Create a new sketch on a plane
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
