//! Camera settings overlay for the 3D viewport

use glam::Vec3;
use rk_renderer::{GizmoMode, GizmoSpace};

use crate::state::{
    AppAction, ExtrudeDirection, SharedAppState, SharedViewportState, SketchAction, SketchTool,
};

/// Render camera settings overlay in the top-right corner (Unity-style)
pub fn render_camera_settings(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    viewport_state: &SharedViewportState,
    show_camera_settings: &mut bool,
    collapsed: &mut bool,
) {
    let panel_width = 180.0;
    let panel_margin = 10.0;

    // When collapsed, show only the expand arrow button
    if *collapsed {
        let arrow_pos = egui::pos2(
            rect.right() - panel_margin - 10.0,
            rect.top() + panel_margin,
        );

        egui::Area::new(egui::Id::new("camera_expand_btn"))
            .fixed_pos(arrow_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 30, 220))
                    .corner_radius(4.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                    .inner_margin(2.0)
                    .show(ui, |ui| {
                        // Expand button (left arrow - to expand from right)
                        let button_size = egui::vec2(10.0, 24.0);
                        let (response, painter) =
                            ui.allocate_painter(button_size, egui::Sense::click());

                        if response.hovered() {
                            painter.rect_filled(
                                response.rect,
                                2.0,
                                egui::Color32::from_rgba_unmultiplied(60, 60, 60, 200),
                            );
                        }

                        let center = response.rect.center();
                        let arrow_height = 5.0;
                        let arrow_width = 3.0;

                        // Left pointing arrow (to expand)
                        let tip = egui::pos2(center.x - arrow_width, center.y);
                        let top = egui::pos2(center.x + arrow_width, center.y - arrow_height);
                        let bottom = egui::pos2(center.x + arrow_width, center.y + arrow_height);

                        let arrow_color = if response.hovered() {
                            egui::Color32::from_gray(200)
                        } else {
                            egui::Color32::from_gray(100)
                        };

                        painter.line_segment([top, tip], egui::Stroke::new(1.0, arrow_color));
                        painter.line_segment([tip, bottom], egui::Stroke::new(1.0, arrow_color));

                        if response.clicked() {
                            *collapsed = false;
                        }

                        response.on_hover_text("Show camera settings");
                    });
            });

        return;
    }

    // When expanded, show the full toolbar with collapse arrow

    // Toggle button at top-right
    let toggle_pos = egui::pos2(
        rect.right() - panel_margin - 42.0, // Adjust for collapse button
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
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;

                        // Collapse toggle button (thin arrow)
                        let button_size = egui::vec2(10.0, 24.0);
                        let (response, painter) =
                            ui.allocate_painter(button_size, egui::Sense::click());

                        // Button background on hover
                        if response.hovered() {
                            painter.rect_filled(
                                response.rect,
                                2.0,
                                egui::Color32::from_rgba_unmultiplied(60, 60, 60, 200),
                            );
                        }

                        // Draw arrow
                        let center = response.rect.center();
                        let arrow_height = 5.0;
                        let arrow_width = 3.0;

                        // Right pointing arrow (to collapse to the right)
                        let tip = egui::pos2(center.x + arrow_width, center.y);
                        let top = egui::pos2(center.x - arrow_width, center.y - arrow_height);
                        let bottom = egui::pos2(center.x - arrow_width, center.y + arrow_height);

                        let arrow_color = if response.hovered() {
                            egui::Color32::from_gray(200)
                        } else {
                            egui::Color32::from_gray(100)
                        };

                        // Draw thin arrow (just lines)
                        painter.line_segment([top, tip], egui::Stroke::new(1.0, arrow_color));
                        painter.line_segment([tip, bottom], egui::Stroke::new(1.0, arrow_color));

                        if response.clicked() {
                            *collapsed = true;
                        }

                        response.on_hover_text("Hide camera settings");

                        // Separator
                        ui.add_space(2.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Camera button
                        let button = egui::Button::new("📷")
                            .selected(*show_camera_settings)
                            .min_size(egui::vec2(24.0, 24.0));
                        if ui.add(button).on_hover_text("Camera Settings").clicked() {
                            *show_camera_settings = !*show_camera_settings;
                        }
                    });
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
/// Returns true if the collapsed state was toggled
pub fn render_gizmo_toggle(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    viewport_state: &SharedViewportState,
    collapsed: &mut bool,
) -> bool {
    let panel_margin = 10.0;
    let mut toggled = false;

    // When collapsed, show only the expand arrow button
    if *collapsed {
        let arrow_pos = egui::pos2(rect.left() + panel_margin, rect.top() + panel_margin);

        egui::Area::new(egui::Id::new("gizmo_expand_btn"))
            .fixed_pos(arrow_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 30, 220))
                    .corner_radius(4.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                    .inner_margin(2.0)
                    .show(ui, |ui| {
                        // Expand button (right arrow)
                        let button_size = egui::vec2(10.0, 24.0);
                        let (response, painter) =
                            ui.allocate_painter(button_size, egui::Sense::click());

                        if response.hovered() {
                            painter.rect_filled(
                                response.rect,
                                2.0,
                                egui::Color32::from_rgba_unmultiplied(60, 60, 60, 200),
                            );
                        }

                        let center = response.rect.center();
                        let arrow_height = 5.0;
                        let arrow_width = 3.0;

                        // Right pointing arrow
                        let tip = egui::pos2(center.x + arrow_width, center.y);
                        let top = egui::pos2(center.x - arrow_width, center.y - arrow_height);
                        let bottom = egui::pos2(center.x - arrow_width, center.y + arrow_height);

                        let arrow_color = if response.hovered() {
                            egui::Color32::from_gray(200)
                        } else {
                            egui::Color32::from_gray(100)
                        };

                        painter.line_segment([top, tip], egui::Stroke::new(1.0, arrow_color));
                        painter.line_segment([tip, bottom], egui::Stroke::new(1.0, arrow_color));

                        if response.clicked() {
                            *collapsed = false;
                            toggled = true;
                        }

                        response.on_hover_text("Show gizmo tools");
                    });
            });

        return toggled;
    }

    // When expanded, show the full toolbar
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

                        drop(vp);

                        // Separator before collapse button
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(2.0);

                        // Collapse toggle button (thin arrow)
                        let button_size = egui::vec2(10.0, 24.0);
                        let (response, painter) =
                            ui.allocate_painter(button_size, egui::Sense::click());

                        // Button background on hover
                        if response.hovered() {
                            painter.rect_filled(
                                response.rect,
                                2.0,
                                egui::Color32::from_rgba_unmultiplied(60, 60, 60, 200),
                            );
                        }

                        // Draw arrow
                        let center = response.rect.center();
                        let arrow_height = 5.0;
                        let arrow_width = 3.0;

                        // Arrow direction: left when expanded (to collapse)
                        let tip = egui::pos2(center.x - arrow_width, center.y);
                        let top = egui::pos2(center.x + arrow_width, center.y - arrow_height);
                        let bottom = egui::pos2(center.x + arrow_width, center.y + arrow_height);

                        let arrow_color = if response.hovered() {
                            egui::Color32::from_gray(200)
                        } else {
                            egui::Color32::from_gray(100)
                        };

                        // Draw thin arrow (just lines)
                        painter.line_segment([top, tip], egui::Stroke::new(1.0, arrow_color));
                        painter.line_segment([tip, bottom], egui::Stroke::new(1.0, arrow_color));

                        if response.clicked() {
                            *collapsed = true;
                            toggled = true;
                        }

                        response.on_hover_text("Hide gizmo tools");
                    });
                });
        });

    toggled
}

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

/// Render extrude dialog overlay
pub fn render_extrude_dialog(ui: &mut egui::Ui, rect: egui::Rect, app_state: &SharedAppState) {
    let dialog_width = 220.0;
    let dialog_height = 200.0;

    // Center the dialog
    let dialog_pos = egui::pos2(
        rect.center().x - dialog_width / 2.0,
        rect.center().y - dialog_height / 2.0,
    );

    egui::Area::new(egui::Id::new("extrude_dialog"))
        .fixed_pos(dialog_pos)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(40, 40, 40, 245))
                .corner_radius(6.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(80)))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.set_width(dialog_width - 24.0);

                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Extrude").size(14.0).strong());
                        ui.add_space(8.0);

                        // Get dialog state
                        let (profile_count, selected_index, error_message) = {
                            let app = app_state.lock();
                            app.cad
                                .editor_mode
                                .sketch()
                                .map(|s| {
                                    (
                                        s.extrude_dialog.profiles.len(),
                                        s.extrude_dialog.selected_profile_index,
                                        s.extrude_dialog.error_message.clone(),
                                    )
                                })
                                .unwrap_or((0, 0, None))
                        };

                        // Profile selection
                        if profile_count > 0 {
                            ui.horizontal(|ui| {
                                ui.label("Profile:");
                                if profile_count == 1 {
                                    ui.label("1 of 1");
                                } else {
                                    // Show profile selector
                                    let selected_text =
                                        format!("{} of {}", selected_index + 1, profile_count);
                                    egui::ComboBox::from_id_salt("profile_selector")
                                        .selected_text(selected_text)
                                        .show_ui(ui, |ui| {
                                            for i in 0..profile_count {
                                                if ui
                                                    .selectable_label(
                                                        i == selected_index,
                                                        format!("Profile {}", i + 1),
                                                    )
                                                    .clicked()
                                                {
                                                    app_state.lock().queue_action(
                                                        AppAction::SketchAction(
                                                            SketchAction::SelectExtrudeProfile {
                                                                profile_index: i,
                                                            },
                                                        ),
                                                    );
                                                }
                                            }
                                        });
                                }
                            });
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
                                                SketchAction::UpdateExtrudeDirection {
                                                    direction: dir,
                                                },
                                            ));
                                        }
                                    }
                                });
                        });

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
                                app_state.lock().queue_action(AppAction::SketchAction(
                                    SketchAction::ExecuteExtrude,
                                ));
                            }
                            if ui.button("Cancel").clicked() {
                                app_state.lock().queue_action(AppAction::SketchAction(
                                    SketchAction::CancelExtrudeDialog,
                                ));
                            }
                        });
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
