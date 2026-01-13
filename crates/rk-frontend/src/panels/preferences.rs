//! Preferences window for application settings

use rk_core::StlUnit;
use rk_renderer::config::RendererConfig;

use crate::config::{EditorConfig, SharedConfig, UiConfig, UiTheme};
use crate::state::{AngleDisplayMode, SharedAppState, SharedViewportState};

/// Current tab in the preferences window
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum PreferencesTab {
    #[default]
    Renderer,
    Editor,
    Interface,
}

/// Preferences window panel
pub struct PreferencesPanel {
    current_tab: PreferencesTab,
}

impl Default for PreferencesPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl PreferencesPanel {
    /// Create a new preferences panel
    pub fn new() -> Self {
        Self {
            current_tab: PreferencesTab::Renderer,
        }
    }

    /// Show the preferences window
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        config: &SharedConfig,
        app_state: &SharedAppState,
        viewport_state: &Option<SharedViewportState>,
        open: &mut bool,
    ) {
        egui::Window::new("Preferences")
            .open(open)
            .resizable(true)
            .default_size([500.0, 450.0])
            .show(ctx, |ui| {
                // Tab bar
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.current_tab,
                        PreferencesTab::Renderer,
                        "Renderer",
                    );
                    ui.selectable_value(&mut self.current_tab, PreferencesTab::Editor, "Editor");
                    ui.selectable_value(
                        &mut self.current_tab,
                        PreferencesTab::Interface,
                        "Interface",
                    );
                });

                ui.separator();

                // Tab content
                egui::ScrollArea::vertical().show(ui, |ui| match self.current_tab {
                    PreferencesTab::Renderer => {
                        self.renderer_tab(ui, config, viewport_state);
                    }
                    PreferencesTab::Editor => {
                        self.editor_tab(ui, config, app_state);
                    }
                    PreferencesTab::Interface => {
                        self.interface_tab(ui, config);
                    }
                });

                ui.separator();

                // Bottom buttons
                ui.horizontal(|ui| {
                    if ui.button("Reset to Defaults").clicked() {
                        config.write().reset_to_defaults();
                        // Apply defaults to renderer
                        if let Some(vp) = viewport_state {
                            let cfg = config.read();
                            vp.lock().renderer.apply_config(&cfg.config().renderer);
                        }
                        // Apply defaults to app state
                        {
                            let cfg = config.read();
                            let mut state = app_state.lock();
                            state.show_part_axes = cfg.config().editor.show_part_axes;
                            state.show_joint_markers = cfg.config().editor.show_joint_markers;
                            state.angle_display_mode = cfg.config().editor.angle_display_mode;
                            state.stl_import_unit = cfg.config().editor.stl_import_unit;
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Save").clicked()
                            && let Err(e) = config.write().save()
                        {
                            tracing::error!("Failed to save config: {}", e);
                        }
                    });
                });
            });
    }

    fn renderer_tab(
        &mut self,
        ui: &mut egui::Ui,
        config: &SharedConfig,
        viewport_state: &Option<SharedViewportState>,
    ) {
        let mut cfg = config.write();
        let renderer_cfg = cfg.config_mut().renderer.clone();
        let mut changed = false;

        // Grid settings
        let mut grid = renderer_cfg.grid.clone();
        ui.collapsing("Grid", |ui| {
            changed |= ui.checkbox(&mut grid.enabled, "Show Grid").changed();
            changed |= ui
                .add(egui::Slider::new(&mut grid.size, 1.0..=100.0).text("Size"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(&mut grid.spacing, 0.1..=10.0).text("Spacing"))
                .changed();

            ui.horizontal(|ui| {
                ui.label("Line Color:");
                let mut color = [
                    (grid.line_color[0] * 255.0) as u8,
                    (grid.line_color[1] * 255.0) as u8,
                    (grid.line_color[2] * 255.0) as u8,
                ];
                if ui.color_edit_button_srgb(&mut color).changed() {
                    grid.line_color = [
                        color[0] as f32 / 255.0,
                        color[1] as f32 / 255.0,
                        color[2] as f32 / 255.0,
                    ];
                    changed = true;
                }
            });
        });

        // Viewport settings
        let mut viewport = renderer_cfg.viewport.clone();
        ui.collapsing("Viewport", |ui| {
            ui.horizontal(|ui| {
                ui.label("Background Color:");
                let mut color = egui::Color32::from_rgba_unmultiplied(
                    (viewport.background_color[0] * 255.0) as u8,
                    (viewport.background_color[1] * 255.0) as u8,
                    (viewport.background_color[2] * 255.0) as u8,
                    (viewport.background_color[3] * 255.0) as u8,
                );
                if ui.color_edit_button_srgba(&mut color).changed() {
                    viewport.background_color = [
                        color.r() as f32 / 255.0,
                        color.g() as f32 / 255.0,
                        color.b() as f32 / 255.0,
                        color.a() as f32 / 255.0,
                    ];
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Anti-aliasing:");
                egui::ComboBox::from_id_salt("msaa")
                    .selected_text(format!("{}x MSAA", viewport.msaa_sample_count))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(&mut viewport.msaa_sample_count, 1, "Off")
                            .changed()
                        {
                            changed = true;
                        }
                        if ui
                            .selectable_value(&mut viewport.msaa_sample_count, 2, "2x")
                            .changed()
                        {
                            changed = true;
                        }
                        if ui
                            .selectable_value(&mut viewport.msaa_sample_count, 4, "4x (default)")
                            .changed()
                        {
                            changed = true;
                        }
                    });
            });
            ui.label("(MSAA changes require restart)");
        });

        // Shadow settings
        let mut shadow = renderer_cfg.shadow.clone();
        ui.collapsing("Shadows", |ui| {
            changed |= ui.checkbox(&mut shadow.enabled, "Enable Shadows").changed();

            ui.horizontal(|ui| {
                ui.label("Quality:");
                egui::ComboBox::from_id_salt("shadow_quality")
                    .selected_text(format!("{}x{}", shadow.map_size, shadow.map_size))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(&mut shadow.map_size, 512, "512 (Low)")
                            .changed()
                        {
                            changed = true;
                        }
                        if ui
                            .selectable_value(&mut shadow.map_size, 1024, "1024 (Medium)")
                            .changed()
                        {
                            changed = true;
                        }
                        if ui
                            .selectable_value(&mut shadow.map_size, 2048, "2048 (High)")
                            .changed()
                        {
                            changed = true;
                        }
                        if ui
                            .selectable_value(&mut shadow.map_size, 4096, "4096 (Ultra)")
                            .changed()
                        {
                            changed = true;
                        }
                    });
            });
            ui.label("(Shadow quality changes require restart)");

            changed |= ui
                .add(egui::Slider::new(&mut shadow.bias, 0.0..=0.02).text("Bias"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(&mut shadow.normal_bias, 0.0..=0.05).text("Normal Bias"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(&mut shadow.softness, 0.0..=3.0).text("Softness"))
                .changed();
        });

        // Lighting settings
        let mut lighting = renderer_cfg.lighting.clone();
        ui.collapsing("Lighting", |ui| {
            changed |= ui
                .add(egui::Slider::new(&mut lighting.intensity, 0.0..=2.0).text("Intensity"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(&mut lighting.ambient_strength, 0.0..=1.0).text("Ambient"))
                .changed();

            ui.horizontal(|ui| {
                ui.label("Light Color:");
                let mut color = [
                    (lighting.color[0] * 255.0) as u8,
                    (lighting.color[1] * 255.0) as u8,
                    (lighting.color[2] * 255.0) as u8,
                ];
                if ui.color_edit_button_srgb(&mut color).changed() {
                    lighting.color = [
                        color[0] as f32 / 255.0,
                        color[1] as f32 / 255.0,
                        color[2] as f32 / 255.0,
                    ];
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Ambient Color:");
                let mut color = [
                    (lighting.ambient_color[0] * 255.0) as u8,
                    (lighting.ambient_color[1] * 255.0) as u8,
                    (lighting.ambient_color[2] * 255.0) as u8,
                ];
                if ui.color_edit_button_srgb(&mut color).changed() {
                    lighting.ambient_color = [
                        color[0] as f32 / 255.0,
                        color[1] as f32 / 255.0,
                        color[2] as f32 / 255.0,
                    ];
                    changed = true;
                }
            });
        });

        // Camera settings
        let mut camera = renderer_cfg.camera.clone();
        ui.collapsing("Camera", |ui| {
            changed |= ui
                .add(egui::Slider::new(&mut camera.fov_degrees, 10.0..=120.0).text("FOV"))
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut camera.pan_sensitivity, 0.0005..=0.01)
                        .text("Pan Sensitivity"),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut camera.zoom_sensitivity, 0.01..=0.5)
                        .text("Zoom Sensitivity"),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut camera.orbit_sensitivity, 0.001..=0.02)
                        .text("Orbit Sensitivity"),
                )
                .changed();
        });

        // Gizmo settings
        let mut gizmo = renderer_cfg.gizmo.clone();
        ui.collapsing("Gizmo", |ui| {
            changed |= ui.checkbox(&mut gizmo.enabled, "Enable Gizmo").changed();
            changed |= ui
                .add(egui::Slider::new(&mut gizmo.scale, 0.5..=2.0).text("Scale"))
                .changed();
        });

        // Apply changes to config and renderer
        if changed {
            let new_config = RendererConfig {
                grid,
                viewport,
                shadow,
                lighting,
                camera,
                gizmo,
            };
            cfg.config_mut().renderer = new_config.clone();

            // Apply to renderer immediately
            if let Some(vp) = viewport_state {
                vp.lock().renderer.apply_config(&new_config);
            }
        }
    }

    fn editor_tab(&mut self, ui: &mut egui::Ui, config: &SharedConfig, app_state: &SharedAppState) {
        let mut cfg = config.write();
        let editor_cfg = cfg.config_mut().editor.clone();
        let mut changed = false;

        let mut show_part_axes = editor_cfg.show_part_axes;
        let mut show_joint_markers = editor_cfg.show_joint_markers;
        let mut angle_display_mode = editor_cfg.angle_display_mode;
        let mut stl_import_unit = editor_cfg.stl_import_unit;

        changed |= ui.checkbox(&mut show_part_axes, "Show Part Axes").changed();
        changed |= ui
            .checkbox(&mut show_joint_markers, "Show Joint Markers")
            .changed();

        ui.horizontal(|ui| {
            ui.label("Angle Display:");
            egui::ComboBox::from_id_salt("angle_mode")
                .selected_text(match angle_display_mode {
                    AngleDisplayMode::Degrees => "Degrees",
                    AngleDisplayMode::Radians => "Radians",
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(
                            &mut angle_display_mode,
                            AngleDisplayMode::Degrees,
                            "Degrees",
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(
                            &mut angle_display_mode,
                            AngleDisplayMode::Radians,
                            "Radians",
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });
        });

        ui.horizontal(|ui| {
            ui.label("STL Import Unit:");
            egui::ComboBox::from_id_salt("stl_unit")
                .selected_text(match stl_import_unit {
                    StlUnit::Meters => "Meters",
                    StlUnit::Millimeters => "Millimeters",
                    StlUnit::Centimeters => "Centimeters",
                    StlUnit::Inches => "Inches",
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(&mut stl_import_unit, StlUnit::Meters, "Meters")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(&mut stl_import_unit, StlUnit::Millimeters, "Millimeters")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(&mut stl_import_unit, StlUnit::Centimeters, "Centimeters")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(&mut stl_import_unit, StlUnit::Inches, "Inches")
                        .changed()
                    {
                        changed = true;
                    }
                });
        });

        if changed {
            cfg.config_mut().editor = EditorConfig {
                show_part_axes,
                show_joint_markers,
                angle_display_mode,
                stl_import_unit,
            };

            // Apply to app state immediately
            let mut state = app_state.lock();
            state.show_part_axes = show_part_axes;
            state.show_joint_markers = show_joint_markers;
            state.angle_display_mode = angle_display_mode;
            state.stl_import_unit = stl_import_unit;
        }
    }

    fn interface_tab(&mut self, ui: &mut egui::Ui, config: &SharedConfig) {
        let mut cfg = config.write();
        let ui_cfg = cfg.config_mut().ui.clone();
        let mut changed = false;

        let mut theme = ui_cfg.theme;
        let mut font_size = ui_cfg.font_size;

        ui.horizontal(|ui| {
            ui.label("Theme:");
            egui::ComboBox::from_id_salt("theme")
                .selected_text(match theme {
                    UiTheme::Dark => "Dark",
                    UiTheme::Light => "Light",
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(&mut theme, UiTheme::Dark, "Dark")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .selectable_value(&mut theme, UiTheme::Light, "Light")
                        .changed()
                    {
                        changed = true;
                    }
                });
        });

        changed |= ui
            .add(egui::Slider::new(&mut font_size, 0.8..=1.5).text("Font Scale"))
            .changed();

        if changed {
            cfg.config_mut().ui = UiConfig { theme, font_size };
        }

        ui.label("(Theme and font changes require restart)");
    }
}
