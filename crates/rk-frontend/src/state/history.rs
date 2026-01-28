//! Undo/Redo history management

use rk_core::Project;

use super::CadState;

/// A snapshot of application state for undo/redo
#[derive(Clone)]
pub struct UndoSnapshot {
    /// Project state
    pub project: Project,
    /// CAD state
    pub cad: CadState,
    /// Description of the action that led to this state
    pub description: String,
}

/// Undo/redo history manager
pub struct UndoHistory {
    /// Stack of states that can be undone
    undo_stack: Vec<UndoSnapshot>,
    /// Stack of states that can be redone
    redo_stack: Vec<UndoSnapshot>,
    /// Maximum number of history entries
    max_history: usize,
}

impl Default for UndoHistory {
    fn default() -> Self {
        Self::new(50)
    }
}

impl UndoHistory {
    /// Create a new history manager with the specified maximum entries
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    /// Save the current state before an action
    pub fn save_state(&mut self, project: &Project, cad: &CadState, description: &str) {
        // Clear redo stack when a new action is performed
        self.redo_stack.clear();

        // Add current state to undo stack
        self.undo_stack.push(UndoSnapshot {
            project: project.clone(),
            cad: cad.clone(),
            description: description.to_string(),
        });

        // Trim history if it exceeds the maximum
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Undo the last action, returning the previous state
    ///
    /// The current state is pushed to the redo stack.
    pub fn undo(
        &mut self,
        current_project: &Project,
        current_cad: &CadState,
    ) -> Option<UndoSnapshot> {
        if let Some(previous) = self.undo_stack.pop() {
            // Save current state to redo stack
            self.redo_stack.push(UndoSnapshot {
                project: current_project.clone(),
                cad: current_cad.clone(),
                description: previous.description.clone(),
            });
            Some(previous)
        } else {
            None
        }
    }

    /// Redo the last undone action, returning the restored state
    ///
    /// The current state is pushed to the undo stack.
    pub fn redo(
        &mut self,
        current_project: &Project,
        current_cad: &CadState,
    ) -> Option<UndoSnapshot> {
        if let Some(next) = self.redo_stack.pop() {
            // Save current state to undo stack
            self.undo_stack.push(UndoSnapshot {
                project: current_project.clone(),
                cad: current_cad.clone(),
                description: next.description.clone(),
            });
            Some(next)
        } else {
            None
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}
