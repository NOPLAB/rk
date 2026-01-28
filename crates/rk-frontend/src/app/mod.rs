//! Main application module

mod dock;
mod menu;
mod overlays;
mod welcome;

use std::sync::Arc;

use egui_dock::{DockArea, DockState, Style};
use parking_lot::Mutex;

use crate::actions::{ActionContext, SharedKernel, dispatch_action};
use crate::config::{SharedConfig, create_shared_config};
use crate::fonts::configure_fonts;
use crate::panels::PreferencesPanel;
use crate::state::AppAction;
use crate::state::{SharedAppState, SharedViewportState, ViewportState, create_shared_state};
use crate::update::{SharedUpdateStatus, UpdateStatus, check_for_updates, create_update_status};
use welcome::WelcomeDialog;

pub use dock::{PanelType, UrdfTabViewer, create_dock_layout};
pub use menu::{MenuAction, render_menu_bar};
pub use overlays::update_overlays;

/// Storage key for tracking first launch
const FIRST_LAUNCH_KEY: &str = "rk_first_launch_completed";

/// Main application
pub struct UrdfEditorApp {
    dock_state: DockState<PanelType>,
    app_state: SharedAppState,
    viewport_state: Option<SharedViewportState>,
    /// CAD kernel for geometry operations
    kernel: SharedKernel,
    update_status: SharedUpdateStatus,
    /// Whether the update notification has been dismissed
    update_dismissed: bool,
    /// Welcome dialog state
    welcome_dialog: WelcomeDialog,
    /// Application configuration
    config: SharedConfig,
    /// Preferences panel
    preferences_panel: PreferencesPanel,
    /// Whether preferences window is open
    preferences_open: bool,
}

impl UrdfEditorApp {
    /// Create a new app
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure fonts with Noto Sans as default
        configure_fonts(&cc.egui_ctx);

        // Initialize CAD kernel
        let kernel: SharedKernel = Arc::from(rk_cad::default_kernel());

        // Load configuration
        let config = create_shared_config();

        // Apply theme from config
        crate::theme::apply_theme(&cc.egui_ctx, &config);

        // Create viewport state if WGPU is available
        let viewport_state = cc.wgpu_render_state.as_ref().map(|render_state| {
            let device = Arc::new(render_state.device.clone());
            let queue = Arc::new(render_state.queue.clone());
            let format = render_state.target_format;

            let mut vp_state = ViewportState::new(device, queue, format);

            // Apply renderer config from saved settings
            {
                let cfg = config.read();
                vp_state.renderer.apply_config(
                    &cfg.config().renderer,
                    &vp_state.device,
                    &vp_state.queue,
                );
            }

            Arc::new(Mutex::new(vp_state))
        });

        // Create app state and apply editor config
        let app_state = create_shared_state();
        {
            let cfg = config.read();
            let mut state = app_state.lock();
            state.show_part_axes = cfg.config().editor.show_part_axes;
            state.show_joint_markers = cfg.config().editor.show_joint_markers;
            state.angle_display_mode = cfg.config().editor.angle_display_mode;
            state.stl_import_unit = cfg.config().editor.stl_import_unit;
        }

        // Create dock layout
        let dock_state = create_dock_layout();

        // Start update check in background
        let update_status = create_update_status();
        check_for_updates(update_status.clone());

        // Check if this is the first launch
        let is_first_launch = cc
            .storage
            .and_then(|s| s.get_string(FIRST_LAUNCH_KEY))
            .is_none();

        Self {
            dock_state,
            app_state,
            viewport_state,
            kernel,
            update_status,
            update_dismissed: false,
            welcome_dialog: WelcomeDialog::new(is_first_launch),
            config,
            preferences_panel: PreferencesPanel::new(),
            preferences_open: false,
        }
    }

    /// Process pending actions
    fn process_actions(&mut self) {
        let actions = self.app_state.lock().take_pending_actions();
        let ctx = ActionContext::new(&self.app_state, &self.viewport_state, &self.kernel);

        for action in actions {
            dispatch_action(action, &ctx);
        }
    }

    /// Handle global keyboard shortcuts
    fn handle_global_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // Ctrl+Z: Undo
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift {
                self.app_state.lock().queue_action(AppAction::Undo);
            }
            // Ctrl+Y or Ctrl+Shift+Z: Redo
            if (i.modifiers.ctrl && i.key_pressed(egui::Key::Y))
                || (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z))
            {
                self.app_state.lock().queue_action(AppAction::Redo);
            }
        });
    }

    /// Show update notification banner
    fn show_update_banner(&mut self, ctx: &egui::Context) {
        let status = self.update_status.lock().clone();

        if let UpdateStatus::UpdateAvailable {
            latest_version,
            release_url,
        } = status
        {
            egui::TopBottomPanel::top("update_banner").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    ui.colored_label(
                        crate::theme::palette::SUCCESS,
                        format!(
                            "New version {} available! (current: {})",
                            latest_version,
                            crate::update::CURRENT_VERSION
                        ),
                    );

                    if ui.hyperlink_to("Download", &release_url).clicked() {
                        #[cfg(not(target_arch = "wasm32"))]
                        if let Err(e) = open::that(&release_url) {
                            tracing::warn!("Failed to open URL: {}", e);
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Dismiss").clicked() {
                            self.update_dismissed = true;
                        }
                    });
                });
            });
        }
    }
}

impl eframe::App for UrdfEditorApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Handle global keyboard shortcuts first
        self.handle_global_shortcuts(ctx);

        // Process pending actions
        self.process_actions();

        // Menu bar
        if let Some(menu_action) = render_menu_bar(ctx, &self.app_state) {
            match menu_action {
                MenuAction::ResetLayout => {
                    self.dock_state = create_dock_layout();
                }
                MenuAction::OpenPreferences => {
                    self.preferences_open = true;
                }
            }
        }

        // Update notification banner
        if !self.update_dismissed {
            self.show_update_banner(ctx);
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
                    config: &self.config,
                },
            );

        // Update overlays when selection changes
        update_overlays(&self.app_state, &self.viewport_state);

        // Welcome dialog (shown on first launch)
        self.welcome_dialog.show(ctx);

        // Preferences window
        if self.preferences_open {
            self.preferences_panel.show(
                ctx,
                &self.config,
                &self.app_state,
                &self.viewport_state,
                &mut self.preferences_open,
            );
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Mark that first launch has completed
        storage.set_string(FIRST_LAUNCH_KEY, "true".to_string());

        // Save configuration to disk
        if let Err(e) = self.config.write().save() {
            tracing::error!("Failed to save config on exit: {}", e);
        }
    }
}
