//! Feature tree panel for parametric CAD modeling
//!
//! Displays the history of sketches and features in a tree view,
//! allowing navigation, editing, and reordering.

use egui::{CollapsingHeader, Ui};
use uuid::Uuid;

use rk_cad::SketchPlane;

use crate::panels::Panel;
use crate::state::{AppAction, SharedAppState, SketchAction};

/// Feature tree panel for CAD modeling
pub struct FeatureTreePanel {
    /// Currently selected item in the tree
    selected: Option<TreeItem>,
    /// Items expanded in the tree
    #[allow(dead_code)]
    expanded: std::collections::HashSet<Uuid>,
}

/// An item in the feature tree
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TreeItem {
    Sketch(Uuid),
    Feature(Uuid),
}

/// Snapshot of sketch data for rendering
struct SketchInfo {
    id: Uuid,
    name: String,
    is_solved: bool,
    dof: u32,
}

/// Snapshot of feature data for rendering
struct FeatureInfo {
    id: Uuid,
    name: String,
    type_name: &'static str,
    is_suppressed: bool,
}

impl FeatureTreePanel {
    pub fn new() -> Self {
        Self {
            selected: None,
            expanded: std::collections::HashSet::new(),
        }
    }
}

impl Default for FeatureTreePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for FeatureTreePanel {
    fn name(&self) -> &str {
        "Features"
    }

    fn ui(&mut self, ui: &mut Ui, app_state: &SharedAppState) {
        // Collect data from state
        let (has_sketches, is_sketch_mode, active_sketch, sketches, features) = {
            let state = app_state.lock();
            let cad = &state.cad;

            let sketches: Vec<SketchInfo> = cad
                .data
                .history
                .sketches()
                .values()
                .map(|s| SketchInfo {
                    id: s.id,
                    name: s.name.clone(),
                    is_solved: s.is_solved(),
                    dof: s.degrees_of_freedom(),
                })
                .collect();

            let features: Vec<FeatureInfo> = cad
                .data
                .history
                .features()
                .map(|f| FeatureInfo {
                    id: f.id(),
                    name: f.name().to_string(),
                    type_name: f.type_name(),
                    is_suppressed: f.is_suppressed(),
                })
                .collect();

            let has_sketches = !sketches.is_empty();
            let is_sketch_mode = cad.is_sketch_mode();
            let active_sketch = cad.editor_mode.sketch().map(|s| s.active_sketch);

            (
                has_sketches,
                is_sketch_mode,
                active_sketch,
                sketches,
                features,
            )
        };

        // Toolbar
        ui.horizontal(|ui| {
            // New sketch button
            if ui
                .button("+ Sketch")
                .on_hover_text("Create new sketch")
                .clicked()
            {
                app_state.lock().queue_action(AppAction::SketchAction(
                    SketchAction::CreateSketch {
                        plane: SketchPlane::xy(),
                    },
                ));
            }

            ui.separator();

            // New feature buttons (disabled when no sketches exist)
            ui.add_enabled_ui(has_sketches, |ui| {
                if ui
                    .button("Extrude")
                    .on_hover_text("Create extrude feature")
                    .clicked()
                {
                    // TODO: Open extrude dialog
                }
            });
        });

        ui.separator();

        // Feature tree
        egui::ScrollArea::vertical()
            .id_salt("feature_tree_scroll")
            .show(ui, |ui| {
                // Sketches section
                CollapsingHeader::new("Sketches")
                    .default_open(true)
                    .show(ui, |ui| {
                        if sketches.is_empty() {
                            ui.weak("No sketches yet.");
                        } else {
                            for sketch in &sketches {
                                let is_active = active_sketch == Some(sketch.id);
                                let is_selected =
                                    self.selected == Some(TreeItem::Sketch(sketch.id));

                                let label = if is_active {
                                    format!("* {} (editing)", sketch.name)
                                } else if sketch.is_solved {
                                    format!("  {} ({})", sketch.name, sketch.dof)
                                } else {
                                    format!("! {} (unsolved)", sketch.name)
                                };

                                let response = ui.selectable_label(is_selected, label);

                                if response.clicked() {
                                    self.selected = Some(TreeItem::Sketch(sketch.id));
                                }

                                if response.double_clicked() && !is_active {
                                    app_state.lock().queue_action(AppAction::SketchAction(
                                        SketchAction::EditSketch {
                                            sketch_id: sketch.id,
                                        },
                                    ));
                                }

                                // Context menu
                                let sketch_id = sketch.id;
                                response.context_menu(|ui| {
                                    if ui.button("Edit").clicked() {
                                        app_state.lock().queue_action(AppAction::SketchAction(
                                            SketchAction::EditSketch { sketch_id },
                                        ));
                                        ui.close();
                                    }
                                    if ui.button("Delete").clicked() {
                                        // TODO: Delete sketch
                                        ui.close();
                                    }
                                });
                            }
                        }
                    });

                // Features section
                CollapsingHeader::new("Features")
                    .default_open(true)
                    .show(ui, |ui| {
                        if features.is_empty() {
                            ui.weak("No features yet.");
                        } else {
                            for feature in &features {
                                let is_selected =
                                    self.selected == Some(TreeItem::Feature(feature.id));
                                let is_suppressed = feature.is_suppressed;

                                let label = if is_suppressed {
                                    format!("  {} [suppressed]", feature.name)
                                } else {
                                    format!("  {} ({})", feature.name, feature.type_name)
                                };

                                let response = ui.selectable_label(is_selected, label);

                                if response.clicked() {
                                    self.selected = Some(TreeItem::Feature(feature.id));
                                }

                                // Context menu
                                response.context_menu(|ui| {
                                    if ui.button("Edit").clicked() {
                                        // TODO: Edit feature
                                        ui.close();
                                    }
                                    if ui
                                        .button(if is_suppressed {
                                            "Unsuppress"
                                        } else {
                                            "Suppress"
                                        })
                                        .clicked()
                                    {
                                        // TODO: Toggle suppression
                                        ui.close();
                                    }
                                    if ui.button("Delete").clicked() {
                                        // TODO: Delete feature
                                        ui.close();
                                    }
                                });
                            }
                        }
                    });
            });

        // Exit sketch mode button (shown when in sketch mode)
        if is_sketch_mode {
            ui.separator();
            if ui.button("Exit Sketch Mode").clicked() {
                app_state
                    .lock()
                    .queue_action(AppAction::SketchAction(SketchAction::ExitSketchMode));
            }
        }
    }
}
