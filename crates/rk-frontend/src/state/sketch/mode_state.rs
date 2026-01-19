//! Sketch mode state

use glam::Vec2;
use uuid::Uuid;

use super::{
    ConstraintToolState, DimensionDialogState, ExtrudeDialogState, InProgressEntity, SketchTool,
};

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
