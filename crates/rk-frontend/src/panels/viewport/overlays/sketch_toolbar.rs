//! Sketch toolbar and plane selection overlay for the viewport

use crate::state::{AppAction, SharedAppState, SketchAction, SketchTool};

/// Render plane selection hint at the top-center of the viewport
pub fn render_plane_selection_hint(ui: &mut egui::Ui, rect: egui::Rect) {
    let panel_margin = 10.0;
    let panel_width = 320.0;

    // Position at top-center
    let hint_pos = egui::pos2(
        rect.center().x - panel_width / 2.0,
        rect.top() + panel_margin,
    );

    egui::Area::new(egui::Id::new("plane_selection_hint"))
        .fixed_pos(hint_pos)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 30, 240))
                .corner_radius(6.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(80)))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.set_width(panel_width - 24.0);

                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("Select a Reference Plane")
                                .size(14.0)
                                .strong(),
                        );
                        ui.add_space(8.0);
                        ui.label("Click on a plane to create a sketch:");
                        ui.add_space(8.0);

                        // Color legend
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 16.0;

                            // XY plane (blue)
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(16.0, 16.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    rect,
                                    2.0,
                                    egui::Color32::from_rgb(77, 128, 230),
                                );
                                ui.label("XY (Top)");
                            });

                            // XZ plane (green)
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(16.0, 16.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    rect,
                                    2.0,
                                    egui::Color32::from_rgb(77, 230, 128),
                                );
                                ui.label("XZ (Front)");
                            });

                            // YZ plane (red)
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(16.0, 16.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    rect,
                                    2.0,
                                    egui::Color32::from_rgb(230, 128, 77),
                                );
                                ui.label("YZ (Side)");
                            });
                        });
                    });
                });
        });
}

/// Render sketch toolbar in the bottom-left corner (visible only in sketch mode)
pub fn render_sketch_toolbar(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    app_state: &SharedAppState,
    current_tool: SketchTool,
) {
    let panel_margin = 10.0;

    // Position at bottom-left
    let toolbar_pos = egui::pos2(
        rect.left() + panel_margin,
        rect.bottom() - panel_margin - 180.0, // Increased height for operations section
    );

    egui::Area::new(egui::Id::new("sketch_toolbar"))
        .fixed_pos(toolbar_pos)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 30, 230))
                .corner_radius(6.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;

                        // Drawing tools section
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Draw")
                                    .small()
                                    .color(egui::Color32::GRAY),
                            );
                        });
                        render_drawing_tools(ui, app_state, current_tool);

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(2.0);

                        // Constraint tools section
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Constrain")
                                    .small()
                                    .color(egui::Color32::GRAY),
                            );
                        });
                        render_constraint_tools(ui, app_state, current_tool);

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(2.0);

                        // Dimension tools section
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Dimension")
                                    .small()
                                    .color(egui::Color32::GRAY),
                            );
                        });
                        render_dimension_tools(ui, app_state, current_tool);

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(2.0);

                        // Operations section
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Operations")
                                    .small()
                                    .color(egui::Color32::GRAY),
                            );
                        });
                        render_operations_tools(ui, app_state);
                    });
                });
        });
}

/// Helper to render a tool button
fn tool_button(
    ui: &mut egui::Ui,
    tool: SketchTool,
    current_tool: SketchTool,
    app_state: &SharedAppState,
) {
    let is_selected = tool == current_tool;
    let btn = egui::Button::new(tool.short_label())
        .selected(is_selected)
        .min_size(egui::vec2(26.0, 26.0));

    if ui.add(btn).on_hover_text(tool.name()).clicked() {
        app_state
            .lock()
            .queue_action(AppAction::SketchAction(SketchAction::SetTool { tool }));
    }
}

/// Render drawing tools row
fn render_drawing_tools(ui: &mut egui::Ui, app_state: &SharedAppState, current_tool: SketchTool) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        // Select tool
        tool_button(ui, SketchTool::Select, current_tool, app_state);

        ui.add_space(4.0);

        // Point and Line
        tool_button(ui, SketchTool::Point, current_tool, app_state);
        tool_button(ui, SketchTool::Line, current_tool, app_state);
    });

    // Rectangle variants with dropdown
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        // Rectangle tools
        tool_button(ui, SketchTool::RectangleCorner, current_tool, app_state);
        tool_button(ui, SketchTool::RectangleCenter, current_tool, app_state);
        tool_button(ui, SketchTool::Rectangle3Point, current_tool, app_state);
    });

    // Circle and Arc variants
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        // Circle tools
        tool_button(ui, SketchTool::CircleCenterRadius, current_tool, app_state);
        tool_button(ui, SketchTool::Circle2Point, current_tool, app_state);
        tool_button(ui, SketchTool::Circle3Point, current_tool, app_state);
    });

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        // Arc tools
        tool_button(ui, SketchTool::ArcCenterStartEnd, current_tool, app_state);
        tool_button(ui, SketchTool::Arc3Point, current_tool, app_state);
    });
}

/// Render constraint tools row
fn render_constraint_tools(
    ui: &mut egui::Ui,
    app_state: &SharedAppState,
    current_tool: SketchTool,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        tool_button(ui, SketchTool::ConstrainCoincident, current_tool, app_state);
        tool_button(ui, SketchTool::ConstrainHorizontal, current_tool, app_state);
        tool_button(ui, SketchTool::ConstrainVertical, current_tool, app_state);
        tool_button(ui, SketchTool::ConstrainParallel, current_tool, app_state);
    });

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        tool_button(
            ui,
            SketchTool::ConstrainPerpendicular,
            current_tool,
            app_state,
        );
        tool_button(ui, SketchTool::ConstrainTangent, current_tool, app_state);
        tool_button(ui, SketchTool::ConstrainEqual, current_tool, app_state);
        tool_button(ui, SketchTool::ConstrainFixed, current_tool, app_state);
    });
}

/// Render dimension tools row
fn render_dimension_tools(ui: &mut egui::Ui, app_state: &SharedAppState, current_tool: SketchTool) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        tool_button(ui, SketchTool::DimensionDistance, current_tool, app_state);
        tool_button(ui, SketchTool::DimensionHorizontal, current_tool, app_state);
        tool_button(ui, SketchTool::DimensionVertical, current_tool, app_state);
    });

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        tool_button(ui, SketchTool::DimensionAngle, current_tool, app_state);
        tool_button(ui, SketchTool::DimensionRadius, current_tool, app_state);
    });
}

/// Render operations tools row (Extrude, etc.)
fn render_operations_tools(ui: &mut egui::Ui, app_state: &SharedAppState) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        // Extrude button
        let extrude_btn = egui::Button::new("Extrude").min_size(egui::vec2(60.0, 26.0));
        if ui
            .add(extrude_btn)
            .on_hover_text("Extrude sketch to 3D")
            .clicked()
        {
            app_state
                .lock()
                .queue_action(AppAction::SketchAction(SketchAction::ShowExtrudeDialog));
        }

        // Exit sketch mode button
        let exit_btn = egui::Button::new("Exit").min_size(egui::vec2(40.0, 26.0));
        if ui.add(exit_btn).on_hover_text("Exit sketch mode").clicked() {
            app_state
                .lock()
                .queue_action(AppAction::SketchAction(SketchAction::ExitSketchMode));
        }
    });
}
