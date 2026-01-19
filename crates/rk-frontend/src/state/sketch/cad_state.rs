//! CAD state management

use rk_cad::{CadData, Sketch, SketchPlane};
use uuid::Uuid;

use super::{EditorMode, SketchModeState};

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
