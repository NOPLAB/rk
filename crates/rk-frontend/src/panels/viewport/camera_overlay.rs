//! Camera settings overlay for the 3D viewport

use glam::Vec3;
use rk_renderer::{GizmoMode, GizmoSpace};

use crate::state::{AppAction, SharedAppState, SharedViewportState, SketchAction, SketchTool};

/// Render camera settings overlay in the top-right corner (Unity-style)
pub fn render_camera_settings(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    viewport_state: &SharedViewportState,
    show_camera_settings: &mut bool,
) {
    let panel_width = 180.0;
    let panel_margin = 10.0;

    // Toggle button at top-right
    let toggle_pos = egui::pos2(
        rect.right() - panel_margin - 28.0,
        rect.top() + panel_margin,
    );

    egui::Area::new(egui::Id::new("camera_toggle"))
        .fixed_pos(toggle_pos)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 30, 220))
                .corner_radius(4.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                .inner_margin(2.0)
                .show(ui, |ui| {
                    let button = egui::Button::new("📷")
                        .selected(*show_camera_settings)
                        .min_size(egui::vec2(24.0, 24.0));
                    if ui.add(button).on_hover_text("Camera Settings").clicked() {
                        *show_camera_settings = !*show_camera_settings;
                    }
                });
        });

    // Camera settings panel (below toggle button)
    if !*show_camera_settings {
        return;
    }

    let panel_pos = egui::pos2(
        rect.right() - panel_width - panel_margin,
        rect.top() + panel_margin + 36.0,
    );

    egui::Area::new(egui::Id::new("camera_settings_overlay"))
        .fixed_pos(panel_pos)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 30, 220))
                .corner_radius(4.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_width(panel_width - 16.0);

                    ui.horizontal(|ui| {
                        ui.strong("Camera");
                    });

                    ui.separator();

                    let mut vp = viewport_state.lock();

                    // FOV slider
                    ui.horizontal(|ui| {
                        ui.label("FOV");
                        ui.add_space(ui.available_width() - 100.0);
                        let mut fov = vp.renderer.camera().fov_degrees();
                        if ui
                            .add(
                                egui::Slider::new(&mut fov, 10.0..=120.0)
                                    .fixed_decimals(0)
                                    .suffix("°")
                                    .show_value(true),
                            )
                            .changed()
                        {
                            vp.renderer.camera_mut().set_fov_degrees(fov);
                        }
                    });

                    // Near plane
                    ui.horizontal(|ui| {
                        ui.label("Near");
                        ui.add_space(ui.available_width() - 100.0);
                        let mut near = vp.renderer.camera().near;
                        if ui
                            .add(
                                egui::DragValue::new(&mut near)
                                    .speed(0.01)
                                    .range(0.001..=100.0)
                                    .suffix(" m"),
                            )
                            .changed()
                        {
                            vp.renderer.camera_mut().set_near(near);
                        }
                    });

                    // Far plane
                    ui.horizontal(|ui| {
                        ui.label("Far");
                        ui.add_space(ui.available_width() - 100.0);
                        let mut far = vp.renderer.camera().far;
                        if ui
                            .add(
                                egui::DragValue::new(&mut far)
                                    .speed(100.0)
                                    .range(10.0..=1000000.0)
                                    .suffix(" m"),
                            )
                            .changed()
                        {
                            vp.renderer.camera_mut().set_far(far);
                        }
                    });

                    ui.separator();

                    // Camera distance info
                    ui.horizontal(|ui| {
                        ui.label("Distance");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("{:.2} m", vp.renderer.camera().distance));
                        });
                    });
                });
        });
}

/// Render gizmo mode toggle in the top-left corner (floating UI)
pub fn render_gizmo_toggle(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    viewport_state: &SharedViewportState,
) {
    let panel_margin = 10.0;

    // Toggle buttons at top-left
    let toggle_pos = egui::pos2(rect.left() + panel_margin, rect.top() + panel_margin);

    egui::Area::new(egui::Id::new("gizmo_toggle"))
        .fixed_pos(toggle_pos)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 30, 220))
                .corner_radius(4.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                .inner_margin(2.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;

                        let mut vp = viewport_state.lock();
                        let current_mode = vp.renderer.gizmo_mode();
                        let current_space = vp.renderer.gizmo_space();

                        // Mode buttons
                        let translate_btn = egui::Button::new("↔")
                            .selected(current_mode == GizmoMode::Translate)
                            .min_size(egui::vec2(24.0, 24.0));
                        if ui.add(translate_btn).on_hover_text("Move (T)").clicked() {
                            vp.renderer.set_gizmo_mode(GizmoMode::Translate);
                        }

                        let rotate_btn = egui::Button::new("⟳")
                            .selected(current_mode == GizmoMode::Rotate)
                            .min_size(egui::vec2(24.0, 24.0));
                        if ui.add(rotate_btn).on_hover_text("Rotate (R)").clicked() {
                            vp.renderer.set_gizmo_mode(GizmoMode::Rotate);
                        }

                        let scale_btn = egui::Button::new("⤢")
                            .selected(current_mode == GizmoMode::Scale)
                            .min_size(egui::vec2(24.0, 24.0));
                        if ui.add(scale_btn).on_hover_text("Scale (S)").clicked() {
                            vp.renderer.set_gizmo_mode(GizmoMode::Scale);
                        }

                        // Separator
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Coordinate space toggle
                        let (space_icon, space_text, next_space) = match current_space {
                            GizmoSpace::Global => ("🌐", "Global (G)", GizmoSpace::Local),
                            GizmoSpace::Local => ("📦", "Local (G)", GizmoSpace::Global),
                        };
                        let space_btn =
                            egui::Button::new(space_icon).min_size(egui::vec2(24.0, 24.0));
                        if ui.add(space_btn).on_hover_text(space_text).clicked() {
                            let queue = vp.queue.clone();
                            vp.renderer.set_gizmo_space(&queue, next_space);
                        }
                    });
                });
        });
}

/// Render axes indicator in the bottom-right corner
pub fn render_axes_indicator(ui: &mut egui::Ui, rect: egui::Rect, yaw: f32, pitch: f32) {
    let painter = ui.painter();
    let axes_center = rect.right_bottom() - egui::vec2(50.0, 50.0);
    let axis_len = 30.0;

    // Calculate camera basis vectors
    let cos_yaw = yaw.cos();
    let sin_yaw = yaw.sin();
    let cos_pitch = pitch.cos();
    let sin_pitch = pitch.sin();

    let forward = Vec3::new(-cos_pitch * cos_yaw, -cos_pitch * sin_yaw, -sin_pitch);
    let world_up = Vec3::Z;
    let right = forward.cross(world_up).normalize();
    let up = right.cross(forward).normalize();

    let project_axis = |world_axis: Vec3| -> (egui::Vec2, f32) {
        let x = world_axis.dot(right);
        let y = world_axis.dot(up);
        let z = world_axis.dot(forward);
        (egui::vec2(x * axis_len, -y * axis_len), z)
    };

    let (x_dir, x_depth) = project_axis(Vec3::X);
    let (y_dir, y_depth) = project_axis(Vec3::Y);
    let (z_dir, z_depth) = project_axis(Vec3::Z);

    let mut axes = [
        (x_depth, x_dir, "X", egui::Color32::from_rgb(255, 68, 68)),
        (y_depth, y_dir, "Y", egui::Color32::from_rgb(68, 255, 68)),
        (z_depth, z_dir, "Z", egui::Color32::from_rgb(68, 68, 255)),
    ];
    axes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    for (_depth, dir, label, color) in axes {
        painter.line_segment(
            [axes_center, axes_center + dir],
            egui::Stroke::new(2.0, color),
        );

        let label_offset = dir.normalized() * 8.0;
        painter.text(
            axes_center + dir + label_offset,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::default(),
            color,
        );
    }
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
        rect.bottom() - panel_margin - 140.0,
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
