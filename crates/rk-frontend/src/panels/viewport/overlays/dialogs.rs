//! Dialogs for the viewport overlay (extrude, dimension)

use rk_cad::BooleanOp;

use crate::state::{AppAction, ExtrudeDirection, SharedAppState, SketchAction};

/// Render extrude dialog overlay (movable window on left-center)
pub fn render_extrude_dialog(ui: &mut egui::Ui, rect: egui::Rect, app_state: &SharedAppState) {
    let dialog_width = 220.0;
    let panel_margin = 20.0;

    // Position at left-center
    let dialog_pos = egui::pos2(rect.left() + panel_margin, rect.center().y - 100.0);

    egui::Window::new("Extrude")
        .id(egui::Id::new("extrude_dialog"))
        .default_pos(dialog_pos)
        .default_width(dialog_width)
        .collapsible(false)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            // Get dialog state
            let (profile_count, selected_indices, error_message) = {
                let app = app_state.lock();
                app.cad
                    .editor_mode
                    .sketch()
                    .map(|s| {
                        (
                            s.extrude_dialog.profiles.len(),
                            s.extrude_dialog.selected_profile_indices.clone(),
                            s.extrude_dialog.error_message.clone(),
                        )
                    })
                    .unwrap_or((0, std::collections::HashSet::new(), None))
            };

            // Profile selection
            if profile_count > 0 {
                ui.label("Profiles:");
                // Show profile checkboxes
                for i in 0..profile_count {
                    let is_selected = selected_indices.contains(&i);
                    let mut checked = is_selected;
                    if ui
                        .checkbox(&mut checked, format!("Profile {}", i + 1))
                        .clicked()
                    {
                        app_state.lock().queue_action(AppAction::SketchAction(
                            SketchAction::ToggleExtrudeProfile { profile_index: i },
                        ));
                    }
                }
                let selected_count = selected_indices.len();
                ui.label(
                    egui::RichText::new(format!(
                        "{} of {} selected",
                        selected_count, profile_count
                    ))
                    .small()
                    .color(egui::Color32::GRAY),
                );
                ui.add_space(4.0);
            } else {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 150, 100),
                    "No closed profiles found",
                );
                ui.add_space(4.0);
            }

            // Distance input
            ui.horizontal(|ui| {
                ui.label("Distance:");
                let mut distance = {
                    let app = app_state.lock();
                    app.cad
                        .editor_mode
                        .sketch()
                        .map(|s| s.extrude_dialog.distance)
                        .unwrap_or(10.0)
                };
                if ui
                    .add(
                        egui::DragValue::new(&mut distance)
                            .speed(0.1)
                            .range(0.001..=10000.0),
                    )
                    .changed()
                {
                    app_state.lock().queue_action(AppAction::SketchAction(
                        SketchAction::UpdateExtrudeDistance { distance },
                    ));
                }
            });

            ui.add_space(4.0);

            // Direction selection
            ui.horizontal(|ui| {
                ui.label("Direction:");
                let current_direction = {
                    let app = app_state.lock();
                    app.cad
                        .editor_mode
                        .sketch()
                        .map(|s| s.extrude_dialog.direction)
                        .unwrap_or(ExtrudeDirection::Positive)
                };

                egui::ComboBox::from_id_salt("extrude_direction")
                    .selected_text(current_direction.name())
                    .show_ui(ui, |ui| {
                        for dir in ExtrudeDirection::all() {
                            if ui
                                .selectable_label(dir == current_direction, dir.name())
                                .clicked()
                            {
                                app_state.lock().queue_action(AppAction::SketchAction(
                                    SketchAction::UpdateExtrudeDirection { direction: dir },
                                ));
                            }
                        }
                    });
            });

            ui.add_space(4.0);

            // Boolean operation selection
            ui.horizontal(|ui| {
                ui.label("Operation:");
                let current_op = {
                    let app = app_state.lock();
                    app.cad
                        .editor_mode
                        .sketch()
                        .map(|s| s.extrude_dialog.boolean_op)
                        .unwrap_or(BooleanOp::New)
                };

                egui::ComboBox::from_id_salt("boolean_op")
                    .selected_text(boolean_op_name(current_op))
                    .show_ui(ui, |ui| {
                        for op in [
                            BooleanOp::New,
                            BooleanOp::Join,
                            BooleanOp::Cut,
                            BooleanOp::Intersect,
                        ] {
                            if ui
                                .selectable_label(op == current_op, boolean_op_name(op))
                                .clicked()
                            {
                                app_state.lock().queue_action(AppAction::SketchAction(
                                    SketchAction::UpdateExtrudeBooleanOp { boolean_op: op },
                                ));
                            }
                        }
                    });
            });

            // Target body selection (only when boolean op is not New)
            let (current_op, current_target, available_bodies) = {
                let app = app_state.lock();
                app.cad
                    .editor_mode
                    .sketch()
                    .map(|s| {
                        (
                            s.extrude_dialog.boolean_op,
                            s.extrude_dialog.target_body,
                            s.extrude_dialog.available_bodies.clone(),
                        )
                    })
                    .unwrap_or((BooleanOp::New, None, Vec::new()))
            };

            if current_op != BooleanOp::New {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Target:");
                    let selected_text = if let Some(target_id) = current_target {
                        available_bodies
                            .iter()
                            .find(|(id, _)| *id == target_id)
                            .map(|(_, name)| name.as_str())
                            .unwrap_or("Unknown")
                    } else {
                        "Select body..."
                    };

                    egui::ComboBox::from_id_salt("target_body")
                        .selected_text(selected_text)
                        .show_ui(ui, |ui| {
                            for (body_id, body_name) in &available_bodies {
                                if ui
                                    .selectable_label(current_target == Some(*body_id), body_name)
                                    .clicked()
                                {
                                    app_state.lock().queue_action(AppAction::SketchAction(
                                        SketchAction::UpdateExtrudeTargetBody {
                                            target_body: Some(*body_id),
                                        },
                                    ));
                                }
                            }
                        });
                });

                if available_bodies.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 150, 100),
                        "No bodies available",
                    );
                }
            }

            // Show error message if any
            if let Some(error) = error_message {
                ui.add_space(4.0);
                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), error);
            }

            ui.add_space(12.0);

            // OK / Cancel buttons
            ui.horizontal(|ui| {
                // Disable OK button if no profiles
                let ok_enabled = profile_count > 0;
                if ui
                    .add_enabled(ok_enabled, egui::Button::new("OK"))
                    .clicked()
                {
                    app_state
                        .lock()
                        .queue_action(AppAction::SketchAction(SketchAction::ExecuteExtrude));
                }
                if ui.button("Cancel").clicked() {
                    app_state
                        .lock()
                        .queue_action(AppAction::SketchAction(SketchAction::CancelExtrudeDialog));
                }
            });
        });
}

/// Render dimension input dialog overlay (movable window)
pub fn render_dimension_dialog(ui: &mut egui::Ui, rect: egui::Rect, app_state: &SharedAppState) {
    let dialog_width = 200.0;
    let panel_margin = 20.0;

    // Position at left-center
    let dialog_pos = egui::pos2(rect.left() + panel_margin, rect.center().y - 60.0);

    egui::Window::new("Dimension")
        .id(egui::Id::new("dimension_dialog"))
        .default_pos(dialog_pos)
        .default_width(dialog_width)
        .collapsible(false)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            // Get dialog state
            let (tool_name, current_value) = {
                let app = app_state.lock();
                app.cad
                    .editor_mode
                    .sketch()
                    .map(|s| {
                        (
                            s.dimension_dialog
                                .tool
                                .map(|t| t.name())
                                .unwrap_or("Dimension"),
                            s.dimension_dialog.value,
                        )
                    })
                    .unwrap_or(("Dimension", 10.0))
            };

            ui.label(tool_name);
            ui.add_space(4.0);

            // Value input
            ui.horizontal(|ui| {
                ui.label("Value:");
                let mut value = current_value;
                if ui
                    .add(
                        egui::DragValue::new(&mut value)
                            .speed(0.1)
                            .range(0.001..=10000.0),
                    )
                    .changed()
                {
                    app_state.lock().queue_action(AppAction::SketchAction(
                        SketchAction::UpdateDimensionValue { value },
                    ));
                }
            });

            ui.add_space(8.0);

            // OK / Cancel buttons
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    app_state.lock().queue_action(AppAction::SketchAction(
                        SketchAction::ConfirmDimensionConstraint,
                    ));
                }
                if ui.button("Cancel").clicked() {
                    app_state
                        .lock()
                        .queue_action(AppAction::SketchAction(SketchAction::CancelDimensionDialog));
                }
            });
        });
}

/// Helper function to get display name for BooleanOp
fn boolean_op_name(op: BooleanOp) -> &'static str {
    match op {
        BooleanOp::New => "New Body",
        BooleanOp::Join => "Join",
        BooleanOp::Cut => "Cut",
        BooleanOp::Intersect => "Intersect",
    }
}
