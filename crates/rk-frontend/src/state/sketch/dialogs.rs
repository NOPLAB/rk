//! Dialog state types for sketch mode

use std::collections::HashSet;

use rk_cad::{BooleanOp, TessellatedMesh, Wire2D};
use uuid::Uuid;

use super::SketchTool;

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
