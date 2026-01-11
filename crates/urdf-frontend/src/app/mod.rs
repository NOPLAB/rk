//! Main application module

mod dock;
mod menu;
mod overlays;

use std::sync::Arc;

use egui_dock::{DockArea, DockState, Style};
use parking_lot::Mutex;

use crate::actions::{ActionContext, dispatch_action};
use crate::state::{SharedAppState, SharedViewportState, ViewportState, create_shared_state};

pub use dock::{PanelType, UrdfTabViewer, create_dock_layout};
pub use menu::{MenuAction, render_menu_bar};
pub use overlays::update_overlays;

/// Main application
pub struct UrdfEditorApp {
    dock_state: DockState<PanelType>,
    app_state: SharedAppState,
    viewport_state: Option<SharedViewportState>,
}

impl UrdfEditorApp {
    /// Create a new app
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Create viewport state if WGPU is available
        let viewport_state = cc.wgpu_render_state.as_ref().map(|render_state| {
            let device = render_state.device.clone();
            let queue = render_state.queue.clone();
            let format = render_state.target_format;

            Arc::new(Mutex::new(ViewportState::new(device, queue, format)))
        });

        // Create dock layout
        let dock_state = create_dock_layout();

        Self {
            dock_state,
            app_state: create_shared_state(),
            viewport_state,
        }
    }

    /// Process pending actions
    fn process_actions(&mut self) {
        let actions = self.app_state.lock().take_pending_actions();
        let ctx = ActionContext::new(&self.app_state, &self.viewport_state);

        for action in actions {
            dispatch_action(action, &ctx);
        }
    }
}

impl eframe::App for UrdfEditorApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Process pending actions
        self.process_actions();

        // Menu bar
        if let Some(menu_action) = render_menu_bar(ctx, &self.app_state) {
            match menu_action {
                MenuAction::ResetLayout => {
                    self.dock_state = create_dock_layout();
                }
            }
        }

        // Dock area
        let render_state = frame.wgpu_render_state();

        DockArea::new(&mut self.dock_state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(
                ctx,
                &mut UrdfTabViewer {
                    app_state: &self.app_state,
                    render_state,
                    viewport_state: &self.viewport_state,
                },
            );

        // Update overlays when selection changes
        update_overlays(&self.app_state, &self.viewport_state);
    }
}
