//! UI panels

mod joint_list;
mod part_list;
mod preferences;
mod properties;
mod viewport;

pub use joint_list::JointListPanel;
pub use part_list::PartListPanel;
pub use preferences::PreferencesPanel;
pub use properties::PropertiesPanel;
pub use viewport::ViewportPanel;

use crate::config::SharedConfig;
use crate::state::{SharedAppState, SharedViewportState};

/// Panel trait for dockable UI panels
pub trait Panel {
    /// Panel name for tab title
    fn name(&self) -> &str;

    /// Draw the panel UI
    fn ui(&mut self, ui: &mut egui::Ui, app_state: &SharedAppState);

    /// Draw with render context (for 3D viewport)
    fn ui_with_render_context(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &SharedAppState,
        render_state: &egui_wgpu::RenderState,
        viewport_state: &SharedViewportState,
        config: &SharedConfig,
    ) {
        // Default: just call ui()
        let _ = (render_state, viewport_state, config);
        self.ui(ui, app_state);
    }

    /// Whether this panel needs render context
    fn needs_render_context(&self) -> bool {
        false
    }
}
