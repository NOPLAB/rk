//! Camera settings overlay for the 3D viewport

use crate::state::SharedViewportState;
use crate::theme::palette;
use rk_renderer::ProjectionMode;

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
                    .fill(palette::overlay_bg(220))
                    .corner_radius(4.0)
                    .stroke(egui::Stroke::new(1.0, palette::BORDER_NORMAL))
                    .inner_margin(2.0)
                    .show(ui, |ui| {
                        // Expand button (left arrow - to expand from right)
                        let button_size = egui::vec2(10.0, 24.0);
                        let (response, painter) =
                            ui.allocate_painter(button_size, egui::Sense::click());

                        if response.hovered() {
                            painter.rect_filled(response.rect, 2.0, palette::BG_HOVER);
                        }

                        let center = response.rect.center();
                        let arrow_height = 5.0;
                        let arrow_width = 3.0;

                        // Left pointing arrow (to expand)
                        let tip = egui::pos2(center.x - arrow_width, center.y);
                        let top = egui::pos2(center.x + arrow_width, center.y - arrow_height);
                        let bottom = egui::pos2(center.x + arrow_width, center.y + arrow_height);

                        let arrow_color = if response.hovered() {
                            palette::TEXT_PRIMARY
                        } else {
                            palette::TEXT_SECONDARY
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
        rect.right() - panel_margin - 50.0, // Adjust for collapse button
        rect.top() + panel_margin,
    );

    egui::Area::new(egui::Id::new("camera_toggle"))
        .fixed_pos(toggle_pos)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .fill(palette::overlay_bg(220))
                .corner_radius(4.0)
                .stroke(egui::Stroke::new(1.0, palette::BORDER_NORMAL))
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
                            painter.rect_filled(response.rect, 2.0, palette::BG_HOVER);
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
                            palette::TEXT_PRIMARY
                        } else {
                            palette::TEXT_SECONDARY
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

                        // Projection mode toggle button
                        {
                            let mut vp = viewport_state.lock();
                            let current_mode = vp.renderer.camera().projection_mode();
                            let (icon, tooltip) = match current_mode {
                                ProjectionMode::Perspective => {
                                    ("🎥", "Perspective (click for Ortho)")
                                }
                                ProjectionMode::Orthographic => {
                                    ("📐", "Orthographic (click for Persp)")
                                }
                            };
                            let button = egui::Button::new(icon).min_size(egui::vec2(24.0, 24.0));
                            if ui.add(button).on_hover_text(tooltip).clicked() {
                                vp.renderer.camera_mut().toggle_projection_mode();
                            }
                        }

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
                .fill(palette::overlay_bg(220))
                .corner_radius(4.0)
                .stroke(egui::Stroke::new(1.0, palette::BORDER_NORMAL))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_width(panel_width - 16.0);

                    ui.horizontal(|ui| {
                        ui.strong("Camera");
                    });

                    ui.separator();

                    let mut vp = viewport_state.lock();

                    // Projection mode
                    ui.horizontal(|ui| {
                        ui.label("Projection");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let current_mode = vp.renderer.camera().projection_mode();
                            let is_perspective = current_mode == ProjectionMode::Perspective;

                            if ui
                                .selectable_label(!is_perspective, "Ortho")
                                .on_hover_text("Orthographic projection")
                                .clicked()
                            {
                                vp.renderer
                                    .camera_mut()
                                    .set_projection_mode(ProjectionMode::Orthographic);
                            }
                            if ui
                                .selectable_label(is_perspective, "Persp")
                                .on_hover_text("Perspective projection")
                                .clicked()
                            {
                                vp.renderer
                                    .camera_mut()
                                    .set_projection_mode(ProjectionMode::Perspective);
                            }
                        });
                    });

                    // FOV slider (only affects perspective, but also affects ortho scale)
                    ui.horizontal(|ui| {
                        let label = match vp.renderer.camera().projection_mode() {
                            ProjectionMode::Perspective => "FOV",
                            ProjectionMode::Orthographic => "FOV",
                        };
                        ui.label(label);
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
